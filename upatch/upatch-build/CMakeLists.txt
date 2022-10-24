# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(upatch_build)

include_directories(
    ${CMAKE_SOURCE_DIR}/
    ${CMAKE_SOURCE_DIR}/insn/
)

file(GLOB HOST_SRC_FILES
    ${CMAKE_SOURCE_DIR}/insn/*.c
    ${CMAKE_SOURCE_DIR}/*.c
)

add_executable(create-diff-object ${HOST_SRC_FILES})
target_link_libraries(create-diff-object elf)