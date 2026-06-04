#!/bin/bash

gdal-config --version
gcc --version
g++ --version
apk add --no-cache cargo geos-dev make rust tiff-dev

mkdir build
