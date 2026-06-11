FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY bin/mlog /usr/local/bin/mlog
RUN chmod +x /usr/local/bin/mlog

WORKDIR /office
COPY app.mlog .
COPY dept/ ./dept/
COPY library/ ./library/
COPY bin/mlog ./bin/mlog
COPY entrypoint.sh ./entrypoint.sh
RUN chmod +x ./entrypoint.sh
RUN mkdir -p /office/data

EXPOSE 10000

ENTRYPOINT ["./entrypoint.sh"]
