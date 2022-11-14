# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(upatch-build)

set(UPATCH_INSTALL_DIR /usr/libexec/syscare)

set(CARGO_CMD cargo build --release)
set(TARGET_DIR "release")

set(UPATCH_BUILD_BIN "upatch-build")

add_custom_target(upatch-build ALL
        COMMENT "Compiling upatch-build..."
        COMMAND CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_CMD}
        COMMAND cp ${CMAKE_CURRENT_BINARY_DIR}/${TARGET_DIR}/${UPATCH_BUILD_BIN} ${CMAKE_CURRENT_BINARY_DIR}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

install(FILES ${CMAKE_CURRENT_BINARY_DIR}/${UPATCH_BUILD_BIN} DESTINATION ${UPATCH_INSTALL_DIR})

