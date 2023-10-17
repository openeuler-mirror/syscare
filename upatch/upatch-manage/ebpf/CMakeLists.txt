# SPDX-License-Identifier: GPL-2.0 OR BSD-3-Clause
cmake_minimum_required(VERSION 3.22)
project(upatch-manager)

find_program(BASH bash HINTS /bin)
find_package(ZLIB)

if(NOT DEFINED BPFTOOL_PATH)
    set(BPFTOOL_PATH "bpftool")
endif()

if(NOT DEFINED CLANG_PATH)
    set(CLANG_PATH "clang")
endif()

if(NOT DEFINED LLVM_STRIP_PATH)
    set(LLVM_STRIP_PATH "llvm-strip")
endif()

if(${CMAKE_SYSTEM_PROCESSOR} MATCHES "x86_64")
    set(ARCH "x86")
elseif(${CMAKE_SYSTEM_PROCESSOR} MATCHES "arm")
    set(ARCH "arm")
elseif(${CMAKE_SYSTEM_PROCESSOR} MATCHES "aarch64")
    set(ARCH "arm64")
elseif(${CMAKE_SYSTEM_PROCESSOR} MATCHES "ppc64le")
    set(ARCH "powerpc")
elseif(${CMAKE_SYSTEM_PROCESSOR} MATCHES "mips")
    set(ARCH "mips")
elseif(${CMAKE_SYSTEM_PROCESSOR} MATCHES "riscv64")
    set(ARCH "riscv")
elseif(${CMAKE_SYSTEM_PROCESSOR} MATCHES "loongarch64")
    set(ARCH "loongarch")
endif()

function(generate_bpf_skel bpf_source bpf_headers bpf_object skel_output)
    add_custom_command(OUTPUT ${bpf_headers}
        COMMAND ${BPFTOOL_PATH} btf dump file
            /sys/kernel/btf/vmlinux format c > ${bpf_headers}
    )
    add_custom_command(OUTPUT ${bpf_object}
        COMMAND ${CLANG_PATH} -g -O2 -target bpf -D__TARGET_ARCH_${ARCH}
                            -I ${CMAKE_CURRENT_BINARY_DIR}
                            -c ${bpf_source}
                            -o ${bpf_object}
        COMMAND ${LLVM_STRIP_PATH} -g ${bpf_object}
        MAIN_DEPENDENCY ${bpf_source}
        DEPENDS ${bpf_headers}
    )
    add_custom_command(OUTPUT ${skel_output}
        COMMAND ${CMAKE_COMMAND} -E env ${BASH} -c
            "${BPFTOOL_PATH} gen skeleton ${bpf_object} > ${skel_output}"
            VERBATIM
            MAIN_DEPENDENCY ${bpf_object}
    )
endfunction(generate_bpf_skel)

set(_skel_output "${CMAKE_PROJECT_NAME}.skel.h")
generate_bpf_skel(
    "${CMAKE_CURRENT_SOURCE_DIR}/${CMAKE_PROJECT_NAME}.bpf.c"
    "vmlinux.h"
    "${CMAKE_PROJECT_NAME}.bpf.o"
    "${_skel_output}"
)
add_compile_options(-g -Wall)
include_directories(${CMAKE_CURRENT_BINARY_DIR})
add_executable(${CMAKE_PROJECT_NAME} ${CMAKE_PROJECT_NAME}.c ${_skel_output})
target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE libbpf.a elf ZLIB::ZLIB)