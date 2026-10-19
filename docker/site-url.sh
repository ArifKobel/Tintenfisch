#!/bin/sh
set -e

sed -i "s|__VITE_SITE_URL__|${VITE_SITE_URL%/}|g" /usr/share/nginx/html/index.html
