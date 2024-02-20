# Set arch related path
set(ARCH_PATH arch/${CMAKE_SYSTEM_PROCESSOR})

# Find source files
file(GLOB SRC_FILES
    *.c
    ${ARCH_PATH}/*.c
)

# Build upatch-manage
add_executable(upatch-manage ${SRC_FILES})

target_include_directories(upatch-manage
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}
        ${ARCH_PATH}
)

target_link_libraries(upatch-manage
    PRIVATE
        elf
)

# Install upatch-manage
install(
    TARGETS
        upatch-manage
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_INSTALL_LIBEXEC_DIR}
)
