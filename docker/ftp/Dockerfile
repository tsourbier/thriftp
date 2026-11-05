FROM alpine:3.20

RUN apk add --no-cache openssl procps shadow vsftpd \
    && adduser -D -s /bin/sh tester \
    && echo 'tester:testpass' | chpasswd

COPY vsftpd.conf /etc/vsftpd-base.conf
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

EXPOSE 21 990 30000-30009

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["plain"]
