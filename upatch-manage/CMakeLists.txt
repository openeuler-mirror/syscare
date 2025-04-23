# SPDX-License-Identifier: GPL-2.0

set(UPATCH_MANAGE "upatch-manage")

set(ARCH_PATH arch/${ARCH})

include_directories(
    arch/${ARCH}/
    ./
)

file(GLOB HOST_SRC_FILES
    arch/${ARCH}/*.c
    *.c
)

add_executable(${UPATCH_MANAGE} ${HOST_SRC_FILES})
target_link_libraries(${UPATCH_MANAGE} elf)

install(
    TARGETS
        ${UPATCH_MANAGE}
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)
