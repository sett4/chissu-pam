# Fedora-based builder for chissu-pam RPM packages
FROM fedora:40

RUN dnf -y update && \
    dnf -y install \
        rpm-build rpmdevtools dnf-plugins-core \
        git curl bzip2 tar \
        gcc clang make pkgconf-pkg-config \
        rust cargo \
        dlib-devel openblas-devel lapack-devel gtk3-devel \
        systemd-devel libudev-devel openssl-devel \
        pam-devel dbus-devel
    # dnf clean all && rm -rf /var/cache/dnf

WORKDIR /workspace
ENV CARGO_HOME=/workspace/.cargo-home

CMD ["/bin/bash"]
