# Import alongside Grocer and Square, without replacing the host configuration.
# All listeners are loopback-only; this module opens no firewall ports.
{ pkgs, ... }:
let
  eins = pkgs.callPackage ./package.nix { };
  web = pkgs.callPackage ./web.nix { };
  nginxConfig = pkgs.writeText "eins-nginx.conf" ''
    daemon off;
    worker_processes 1;
    pid /run/eins-web/nginx.pid;
    error_log stderr;
    events { worker_connections 1024; }
    http {
      include ${pkgs.nginx}/conf/mime.types;
      types { application/manifest+json webmanifest; }
      default_type application/octet-stream;
      access_log off;
      client_body_temp_path /run/eins-web/client_body;
      proxy_temp_path /run/eins-web/proxy;
      fastcgi_temp_path /run/eins-web/fastcgi;
      uwsgi_temp_path /run/eins-web/uwsgi;
      scgi_temp_path /run/eins-web/scgi;
      server {
        listen 127.0.0.1:8082;
        server_name _;
        root ${web}/share/eins;
        index index.html;
        location /api/ {
          proxy_pass http://127.0.0.1:8080/;
          proxy_http_version 1.1;
          proxy_set_header Connection "";
          proxy_set_header Host $host;
          proxy_buffering off;
          proxy_cache off;
          proxy_read_timeout 1h;
          add_header Cache-Control "no-store" always;
        }
        location / {
          try_files $uri $uri/ =404;
          add_header Cache-Control "no-cache" always;
        }
      }
    }
  '';
in
{
  systemd.services.eins = {
    description = "Eins game server";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${eins}/bin/eins_server 127.0.0.1:8080";
      DynamicUser = true;
      Restart = "on-failure";
      RestartSec = 5;
      NoNewPrivileges = true;
      PrivateTmp = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      UMask = "0077";
    };
  };

  # Dedicated nginx, not services.nginx: no shared/default public listener.
  systemd.services.eins-web = {
    description = "Eins website and API proxy";
    wantedBy = [ "multi-user.target" ];
    wants = [ "eins.service" ];
    after = [ "eins.service" ];
    serviceConfig = {
      ExecStart = "${pkgs.nginx}/bin/nginx -c ${nginxConfig} -p /run/eins-web -e stderr";
      DynamicUser = true;
      RuntimeDirectory = "eins-web";
      RuntimeDirectoryMode = "0700";
      Restart = "on-failure";
      RestartSec = 5;
      NoNewPrivileges = true;
      PrivateTmp = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      UMask = "0077";
    };
  };

  systemd.services.eins-tunnel = {
    description = "Cloudflare Tunnel for Eins";
    wantedBy = [ "multi-user.target" ];
    wants = [ "network-online.target" "eins-web.service" ];
    after = [ "network-online.target" "eins-web.service" ];
    unitConfig.ConditionPathExists = "/etc/eins/tunnel-token";
    serviceConfig = {
      ExecStart = "${pkgs.cloudflared}/bin/cloudflared tunnel --no-autoupdate --metrics 127.0.0.1:20243 run --token-file=%d/tunnel-token";
      LoadCredential = "tunnel-token:/etc/eins/tunnel-token";
      DynamicUser = true;
      Restart = "on-failure";
      RestartSec = 5;
      NoNewPrivileges = true;
      PrivateTmp = true;
      ProtectSystem = "strict";
      ProtectHome = true;
    };
  };
}
