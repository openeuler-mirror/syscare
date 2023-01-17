# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(upatch-tool)

set(UPATCH_INSTALL_DIR /usr/libexec/syscare)

include_directories(
    ../kmod
    ./
)

file(GLOB TOOL_SRC_FILES
    upatch-tool.c
    upatch-resolve.c
)

add_executable(upatch-tool ${TOOL_SRC_FILES})

install(TARGETS upatch-tool DESTINATION ${UPATCH_INSTALL_DIR})

include_directories(
    ../upatch-diff
)

file(GLOB NOTES_SRC_FILES
    upatch-notes.c
)

add_executable(upatch-notes ${NOTES_SRC_FILES})
target_link_libraries(upatch-notes elf)

install(TARGETS upatch-notes DESTINATION ${UPATCH_INSTALL_DIR})
