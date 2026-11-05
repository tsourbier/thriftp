#!/bin/sh
set -eu

MODE="${1:-plain}"
CERT=/etc/ssl/private/vsftpd.pem
PASV_ADDRESS="${PASV_ADDRESS:-127.0.0.1}"
PASV_MIN_PORT="${PASV_MIN_PORT:-30000}"
PASV_MAX_PORT="${PASV_MAX_PORT:-30009}"

mkdir -p /var/run/vsftpd/empty /var/log/vsftpd

{
  cat /etc/vsftpd-base.conf
  echo "pasv_address=${PASV_ADDRESS}"
  echo "pasv_min_port=${PASV_MIN_PORT}"
  echo "pasv_max_port=${PASV_MAX_PORT}"
  if [ "$MODE" != "plain" ]; then
    cat /certs/server.crt /certs/server.key > "$CERT"
    chmod 600 "$CERT"
    echo "ssl_enable=YES"
    echo "allow_anon_ssl=NO"
    echo "force_local_logins_ssl=NO"
    echo "force_local_data_ssl=NO"
    echo "ssl_tlsv1=NO"
    echo "ssl_sslv2=NO"
    echo "ssl_sslv3=NO"
    echo "require_ssl_reuse=NO"
    echo "ssl_ciphers=HIGH"
    echo "rsa_cert_file=${CERT}"
    echo "rsa_private_key_file=${CERT}"
  fi
  if [ "$MODE" = "implicit" ]; then
    echo "listen_port=990"
    echo "implicit_ssl=YES"
    echo "force_local_logins_ssl=YES"
    echo "force_local_data_ssl=YES"
  fi
} > /etc/vsftpd.conf

exec /usr/sbin/vsftpd /etc/vsftpd.conf
