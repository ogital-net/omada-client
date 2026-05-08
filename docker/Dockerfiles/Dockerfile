FROM ubuntu:22.04 AS build-env

RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && rm -rf /var/lib/apt/lists/*

ADD https://static.tp-link.com/upload/software/2026/202604/20260429/Omada_Network_Application_v6.2.10.17_linux_x64_20260428102037.tar.gz /build/

WORKDIR /build
RUN arch=$(uname -m) && \
    if [ "$arch" = "x86_64" ]; then \
        url="https://fastdl.mongodb.org/linux/mongodb-linux-x86_64-ubuntu2204-8.2.7.tgz"; \
    elif [ "$arch" = "aarch64" ]; then \
        url="https://fastdl.mongodb.org/linux/mongodb-linux-aarch64-ubuntu2204-8.2.7.tgz"; \
    else \
        echo "Unsupported architecture: $arch" && exit 1; \
    fi && \
    curl -fsSL "$url" -o mongodb.tgz

WORKDIR /build
RUN tar -xzf Omada_Network_Application_v6.2.10.17_linux_x64_20260428102037.tar.gz && \
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
RUN cp -r /build/mongodb-linux-*-ubuntu2204-8.2.7 ./mongodb && \
    chmod 755 /opt/mongodb/bin/mongod && \
    ln -sf /opt/mongodb/bin/mongod /opt/tplink/EAPController/bin/mongod

FROM ubuntu:22.04

ENV LANG=C.UTF-8
ENV JAVA_HOME=/usr/lib/jvm/java-21-amazon-corretto

ARG version=21.0.11.10-1

RUN set -ux \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        curl ca-certificates gnupg software-properties-common \
    && curl -fL https://apt.corretto.aws/corretto.key | apt-key add - \
    && add-apt-repository 'deb https://apt.corretto.aws stable main' \
    && mkdir -p /usr/share/man/man1 \
    && apt-get update \
    && apt-get install -y java-21-amazon-corretto-jdk=1:$version binutils \
    && jlink --add-modules "$(java --list-modules | sed -e 's/@[0-9].*$/,/' | tr -d \\n)" --no-man-pages --no-header-files --strip-debug --output /opt/corretto-slim \
    && apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false \
            gnupg software-properties-common binutils java-21-amazon-corretto-jdk=1:$version \
    && mkdir -p /usr/lib/jvm \
    && mv /opt/corretto-slim /usr/lib/jvm/java-21-amazon-corretto \
    && jdk_tools="java keytool rmid rmiregistry javac jaotc jlink jmod jhsdb jar jarsigner javadoc javap jcmd jconsole jdb jdeps jdeprscan jimage jinfo jmap jps jrunscript jshell jstack jstat jstatd serialver" \
    && priority=$(echo "1${version}" | sed "s/\(\.\|-\)//g") \
    && for i in ${jdk_tools}; do \
          update-alternatives --install /usr/bin/$i $i /usr/lib/jvm/java-21-amazon-corretto/bin/$i ${priority}; \
       done


COPY --from=build-env /opt /opt

VOLUME /opt/tplink/EAPController/data

EXPOSE 8088 8043 19810/udp 29810/udp 29811 29812 29813 29814 29815 29816

WORKDIR /opt/tplink/EAPController/lib
ENTRYPOINT ["/usr/bin/java", \
    "-cp", "/opt/tplink/EAPController/lib/*:/opt/tplink/EAPController/properties", \
    "-server", "-XX:MaxHeapFreeRatio=60", "-XX:MinHeapFreeRatio=30", "-Djava.awt.headless=true", "-Djdk.lang.Process.launchMechanism=vfork", \
    "com.tplink.smb.omada.starter.OmadaLinuxMain", "start"]
#CMD [""]
