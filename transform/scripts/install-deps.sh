#!/usr/bin/env bash

apt install -y \
    libcairo2-dev \
    poppler-data \
    cmake \
    libjpeg-dev \
    libopenjp2-7-dev \
    libboost-all-dev

wget "https://poppler.freedesktop.org/poppler-0.87.0.tar.xz" && \
    tar -xvf poppler-0.87.0.tar.xz && \
    cd poppler-0.87.0 && \
    mkdir -p build && cd build && \
    cmake .. && make  && make install
