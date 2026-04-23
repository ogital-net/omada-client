FROM eclipse-temurin:21 AS build-env

ENV JAVA_HOME=/opt/java/openjdk

RUN apt-get update && apt-get install -y --no-install-recommends unzip curl && rm -rf /var/lib/apt/lists/*

ADD https://static.tp-link.com/upload/beta/2026/202604/20260417/Omada_Network_Application_v6.2.10.15_linux_x64_20260417100347.tar.gz.zip /build/

WORKDIR /build
RUN arch=$(uname -m) && \
    if [ "$arch" = "x86_64" ]; then \
        url="https://fastdl.mongodb.org/linux/mongodb-linux-x86_64-ubuntu2404-8.2.7.tgz"; \
    elif [ "$arch" = "aarch64" ]; then \
        url="https://fastdl.mongodb.org/linux/mongodb-linux-aarch64-ubuntu2404-8.2.7.tgz"; \
    else \
        echo "Unsupported architecture: $arch" && exit 1; \
    fi && \
    curl -fsSL "$url" -o mongodb.tgz

WORKDIR /build
RUN unzip Omada_Network_Application_v6.2.10.15_linux_x64_20260417100347.tar.gz.zip && \
    tar -xzf Omada_Network_Application_v6.2.10.15_linux_x64_20260417100347.tar.gz && \
    tar -xzf mongodb.tgz && \
    find . -ls

WORKDIR /opt/tplink/EAPController
RUN extracted=$(find /build -maxdepth 1 -mindepth 1 -type d -name 'Omada_*' | head -1) && \
    echo "Using extracted dir: $extracted" && \
    cp -r "$extracted/data" ./ && \
    cp -r "$extracted/properties" ./ && \
    cp -r "$extracted/lib" ./ && \
    mkdir bin && \
    mkdir logs && \
    mkdir work && \
    ln -sf /dev/stdout /opt/tplink/EAPController/logs/server.log

WORKDIR /opt
RUN cp -r /build/mongodb-linux-*-ubuntu2404-8.2.7 ./mongodb && \
    chmod 755 /opt/mongodb/bin/mongod && \
    ln -sf /opt/mongodb/bin/mongod /opt/tplink/EAPController/bin/mongod

FROM eclipse-temurin:21-jre
COPY --from=build-env /opt /opt

VOLUME /opt/tplink/EAPController/data

EXPOSE 8088 8043 19810/udp 29810/udp 29811 29812 29813 29814 29815 29816

WORKDIR /opt/tplink/EAPController/lib
ENTRYPOINT ["/opt/java/openjdk/bin/java", \
    "-cp", "/opt/tplink/EAPController/lib/*:/opt/tplink/EAPController/properties", \
    "-server", "-XX:MaxHeapFreeRatio=60", "-XX:MinHeapFreeRatio=30", "-Djava.awt.headless=true", \
    "com.tplink.smb.omada.starter.OmadaLinuxMain", "start"]
#CMD [""]
