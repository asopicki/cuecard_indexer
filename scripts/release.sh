#!/bin/bash
# 
# Simple release script for cuecard_indexer
# 
# 

#Use branch for release or master as default
BRANCH=${1:master}
BASE_DIR=`realpath $0`
BASE_DIR=`dirname ${BASE_DIR}`
BASE_DIR=`dirname ${BASE_DIR}`

RELEASE_DIR="~/tools/"
BUILD_DIR=${BASE_DIR}/build
TARGET_FILE=target/release/cuecard_indexer

#create build directory
mkdir -p ${BUILD_DIR}

cd ${BUILD_DIR}

#clone git repository into build directory
git clone ${BASE_DIR} .

#build release
cargo +beta build --release

#optimize file size by stripping debug symbols
strip -S ${BUILD_DIR}/${TARGET_FILE}

#copy executable to release directory
cp ${BUILD_DIR}/${TARGET_FILE} ${RELEASE_DIR}
