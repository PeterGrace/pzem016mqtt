FROM docker.io/ubuntu:20.04
ARG TARGETARCH

RUN mkdir -p /opt/app
WORKDIR /opt/app
COPY ./tools/target_arch.sh /opt/app
COPY config.yaml /opt/app/config.yaml
RUN --mount=type=bind,target=/context \
 cp /context/target/$(/opt/app/target_arch.sh)/release/pzem016mqtt /opt/app/pzem016mqtt 
CMD ["/opt/app/pzem016mqtt"]
EXPOSE 8443
