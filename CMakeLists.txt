# SPDX-License-Identifier: GPL-2.0

cmake_minimum_required(VERSION 3.14)
project(syscare)

if (CMAKE_SYSTEM_PROCESSOR MATCHES "x86_64")
	add_subdirectory(upatch)
endif()

add_subdirectory(build)
add_subdirectory(manager)
