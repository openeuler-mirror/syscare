# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(upatch-tool)

set(UPATCH_INSTALL_DIR /usr/libexec/syscare)

set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wall -Werror")

include_directories(
    ../kmod
    ../upatch_build
)

file(GLOB HOST_SRC_FILES
    *.c
)

add_executable(upatch-tool ${HOST_SRC_FILES})

install(TARGETS upatch-tool DESTINATION ${UPATCH_INSTALL_DIR})
