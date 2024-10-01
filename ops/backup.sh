#!/bin/bash
set -e

echo "Creating backup"
cd /srv/vzdv
sqlite3 vzdv_data.sqlite ".backup vzdv_data.sqlite.backup"
b2v3 file upload zdv-wm-files vzdv_data.sqlite.backup vzdv_data.sqlite
rm vzdv_data.sqlite.backup
echo "Done"
