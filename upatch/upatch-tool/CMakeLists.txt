# SPDX-License-Identifier: GPL-2.0

add_subdirectory(ko)

set(CMAKE_C_FLAGS "-fPIE")
include_directories(
    ./
    ./ko/include
)

file(GLOB UPATCH_LIB_SRC_FILES
    *.c
)

add_library(upatch-tool-lib ${UPATCH_LIB_SRC_FILES})
