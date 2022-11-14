# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(upatch-diff)

set(UPATCH_INSTALL_DIR /usr/libexec/syscare)

include_directories(
    ./
    ./insn/
)

file(GLOB HOST_SRC_FILES
    insn/*.c
    *.c
)

add_executable(create-diff-object ${HOST_SRC_FILES})
target_link_libraries(create-diff-object elf)

install(TARGETS create-diff-object DESTINATION ${UPATCH_INSTALL_DIR})
