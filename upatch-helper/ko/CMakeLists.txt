# Build upatch-helper kernel module

# Set target
set(UPATCH_HELPER_KMOD "upatch_helper.ko")

# Detect kernel source path
if (DEFINED KERNEL_VERSION)
    set(KERNEL_SOURCE_PATH "/lib/modules/${KERNEL_VERSION}/build")
    set(UPATCH_HELPER_KMOD_BUILD_CMD make module_version=${BUILD_VERSION} kernel=${KERNEL_SOURCE_PATH})
else()
    set(UPATCH_HELPER_KMOD_BUILD_CMD make module_version=${BUILD_VERSION})
endif()

# Build kernel module
add_custom_target(upatch-helper-kmod ALL
    COMMENT           "Building kernel module upatch_helper..."
    BYPRODUCTS        ${UPATCH_HELPER_KMOD}
    COMMAND           ${UPATCH_HELPER_KMOD_BUILD_CMD}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Install kernel module
install(
    FILES
        ${UPATCH_HELPER_KMOD}
    PERMISSIONS
        OWNER_WRITE OWNER_READ
        GROUP_READ
        WORLD_READ
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)
