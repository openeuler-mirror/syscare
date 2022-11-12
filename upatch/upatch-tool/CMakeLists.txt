# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(upatch-tool)

include_directories(
    ${CMAKE_SOURCE_DIR}/../kmod
    ${CMAKE_SOURCE_DIR}/../upatch_build
)

file(GLOB HOST_SRC_FILES
    ${CMAKE_SOURCE_DIR}/*.c
)

add_executable(upatch-tool ${HOST_SRC_FILES})