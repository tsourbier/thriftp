FROM debian:13-slim

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get upgrade -y \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        netcat-openbsd openssh-server procps \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -s /bin/bash tester \
    && echo 'tester:testpass' | chpasswd \
    && mkdir -p /run/sshd /home/tester/.ssh

COPY sshd_config /etc/ssh/sshd_config
COPY keys/ssh_host_ed25519_key /etc/ssh/ssh_host_ed25519_key
COPY keys/ssh_host_ed25519_key.pub /etc/ssh/ssh_host_ed25519_key.pub
COPY keys/client_ed25519.pub /home/tester/.ssh/authorized_keys

RUN chmod 600 /etc/ssh/ssh_host_ed25519_key /home/tester/.ssh/authorized_keys \
    && chown -R tester:tester /home/tester/.ssh

EXPOSE 22

CMD ["/usr/sbin/sshd", "-D", "-e"]
