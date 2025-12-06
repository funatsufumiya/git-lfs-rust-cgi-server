#!/bin/bash
cd "$(dirname "$0")"
mkdir -p cgi-bin
python3 -m http.server --cgi
