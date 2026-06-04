#!/bin/bash

gdal-config --version
gcc --version
g++ --version
apk add --no-cache cargo clang-dev geos-dev make rust tiff-dev

mkdir build
