# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(upatch)

SET(UPATCH_VERSION ${SYSCARE_BUILD_VERSION})

add_subdirectory(upatch-build)
add_subdirectory(upatch-diff)
add_subdirectory(upatch-tool)
add_subdirectory(kmod)
