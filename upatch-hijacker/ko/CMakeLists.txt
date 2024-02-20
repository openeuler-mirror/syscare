# Set target
set(KMOD_NAME "upatch_hijacker.ko")

# Set kernel source path
if (DEFINED KERNEL_VERSION)
    set(KERNEL_DIR "/lib/modules/${KERNEL_VERSION}/build")
    set(KMOD_BUILD_CMD make MODULE_VERSION=${BUILD_VERSION} KERNEL_DIR=${KERNEL_DIR})
else()
    set(KMOD_BUILD_CMD make MODULE_VERSION=${BUILD_VERSION})
endif()

# Build kernel module
add_custom_target(upatch-hijacker-kmod ALL
    COMMENT           "Building kernel module upatch-hijacker..."
    COMMAND           ${KMOD_BUILD_CMD}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Install kernel module
install(
    FILES
        ${KMOD_NAME}
    PERMISSIONS
        OWNER_WRITE OWNER_READ
        GROUP_READ
        WORLD_READ
    DESTINATION
        ${SYSCARE_INSTALL_LIBEXEC_DIR}
)
