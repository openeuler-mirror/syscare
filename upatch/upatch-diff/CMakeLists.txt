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

add_executable(upatch-diff ${HOST_SRC_FILES})
target_link_libraries(upatch-diff elf)

install(TARGETS upatch-diff DESTINATION ${UPATCH_INSTALL_DIR})
