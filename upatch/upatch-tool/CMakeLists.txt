# SPDX-License-Identifier: GPL-2.0

include_directories(
    ./
)

file(GLOB UPATCH_LIB_SRC_FILES
    *.c
)

add_library(upatch-tool-lib ${UPATCH_LIB_SRC_FILES})
