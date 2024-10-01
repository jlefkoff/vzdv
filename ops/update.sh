#!/bin/bash
set -e

if test -f 'vzdv-site.new'
then
  echo "Updating vzdv-site"
  caddy reload --config /etc/caddy/Caddyfile-maintenance
  systemctl stop vzdv-site
  mv vzdv-site.new vzdv-site
  systemctl start vzdv-site
  caddy reload --config /etc/caddy/Caddyfile-live
  echo "Done"
fi

if test -f 'vzdv-tasks.new'
then
  echo "Updating vzdv-tasks"
  systemctl stop vzdv-tasks
  mv vzdv-tasks.new vzdv-tasks
  systemctl start vzdv-tasks
  echo "Done"
fi

if test -f 'vzdv-bot.new'
then
  echo "Updating vzdv-bot"
  systemctl stop vzdv-bot
  mv vzdv-bot.new vzdv-bot
  systemctl start vzdv-bot
  echo "Done"
fi
