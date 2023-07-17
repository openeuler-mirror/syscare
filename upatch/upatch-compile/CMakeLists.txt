cmake_minimum_required(VERSION 3.24)
project(upatch-hijacker)

add_executable(upatch-gnu-cc1-hijacker gnu-cc1-hijacker.c)
add_executable(upatch-gnu-as-hijacker gnu-as-hijacker.c)

include_directories(./lib)
include_directories(./common)
add_library(upatch SHARED ./lib/upatch.c)

