
using System.Buffers;
using System.IO.Pipelines;
using System.Net;
using System.Net.Sockets;
using System.Reactive;
using System.Reactive.Concurrency;
using System.Reactive.Linq;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Nito.AsyncEx;

var builder = Host.CreateDefaultBuilder();
builder.ConfigureServices(services =>
{
    services.AddHostedService<Forwarder>();
    services.AddHttpClient();
});

using var host = builder.Build();
host.Run();

file class Forwarder(HttpClient httpClient, IConfiguration configuration, ILogger<Forwarder> logger) : BackgroundService
{
    private async Task DoWork(CancellationToken stoppingToken)
    {

        var scheduler = new SynchronizationContextScheduler(SynchronizationContext.Current!);

        var targetPort = Environment.GetEnvironmentVariable("HTTP_TUNNEL_PORT")?.GetInt()
                  ?? 3128;
        var targetHost = Environment.GetEnvironmentVariable("HTTP_TUNNEL_ADDRESS")?.GetIpAddress() ?? throw new Exception();
        var target = new IPEndPoint(targetHost, targetPort);
        using var listener = TcpListener.Create(3128);

        listener.Start();

        var pingEndpoint = Environment.GetEnvironmentVariable("PING_ENDPOINT")!;

        var poller = Observable.Create<Unit>(async (observer, cancel) =>
            {
                try
                {
                    while (!cancel.IsCancellationRequested)
                    {
                        using var _ = await httpClient.GetAsync(pingEndpoint, cancel);
                        await Task.Delay(TimeSpan.FromSeconds(10), cancel);
                    }
                }
                finally
                {
                    logger.LogInformation("Poller stopped");
                }
            })
            .SubscribeOn(scheduler)
            .Publish()
            .RefCount(disconnectDelay: TimeSpan.FromSeconds(10), scheduler: TaskPoolScheduler.Default);

        while (!stoppingToken.IsCancellationRequested)
        {
            var client = await listener.AcceptTcpClientAsync(stoppingToken);
            var enableNagle = configuration.GetValue<bool?>("EnableNagle") ?? true;
            client.NoDelay = !enableNagle;

            logger.LogInformation("Client connected from {ClientIP} with {EnableNagle}", client.Client.RemoteEndPoint?.ToString(), enableNagle);
            _ = scheduler.ScheduleAsync(async (_, _) =>
            {
                using var pollerGuard = poller.Subscribe();

                using var server = new TcpClient();
                server.NoDelay = !enableNagle;

                await server.ConnectAsync(target, stoppingToken);
                await using var clientStream = client.GetStream();
                await using var serverStream = server.GetStream();

                await Task.WhenAny(
                    clientStream.CopyToAsync(serverStream, stoppingToken),
                    serverStream.CopyToAsync(clientStream, stoppingToken));
            });
        }

        await Task.Delay(Timeout.Infinite, stoppingToken);
    }

    protected override Task ExecuteAsync(CancellationToken stoppingToken)
    {
        return Task.Run(() =>
        {
            AsyncContext.Run(() => DoWork(stoppingToken));
        });
    }
}

file static class Helper
{
    public static int? GetInt(this string? value)
    {
        if (value == null) return null;

        return int.TryParse(value, out var result) ? result : null;
    }

    public static IPAddress? GetIpAddress(this string? value)
    {
        if (value == null) return null;
        return IPAddress.TryParse(value, out var result) ? result : null;
    }
}