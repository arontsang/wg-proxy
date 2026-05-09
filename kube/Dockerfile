FROM --platform=$TARGETPLATFORM alpine:latest

RUN apk add --no-cache curl socat
COPY --chmod=555 /scripts/ /scripts/
ENV PING_ENDPOINT="https://foobar.region.azurecontainerapps.io/Sleep?seconds=1"
ENV PREWARM_ENDPOINT="https://foobar.region.azurecontainerapps.io/Sleep?seconds=0"
ENV HTTP_TUNNEL_ADDRESS=10.11.209.100
ENV HTTP_TUNNEL_PORT=3128

ENTRYPOINT ["/scripts/proxy.sh"]
