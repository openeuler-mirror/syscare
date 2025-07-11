# SPDX-License-Identifier: MulanPSL-2.0

cmake_minimum_required(VERSION 3.14)

project(syscare)

# Arch
EXECUTE_PROCESS(COMMAND uname -m
                OUTPUT_VARIABLE ARCH
                OUTPUT_STRIP_TRAILING_WHITESPACE)

# Includes
include(GNUInstallDirs)
find_package(Git QUIET)

# Build type
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE RelWithDebInfo)
endif()

# Build version
if(NOT DEFINED BUILD_VERSION)
    execute_process(
        COMMAND sh -c "cat syscare/Cargo.toml | grep -F 'version' | head -n 1 | awk -F '\"' '{print $2}'"
        OUTPUT_VARIABLE BUILD_VERSION
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_QUIET
        WORKING_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}
    )
endif()

if(GIT_FOUND)
    execute_process(
        COMMAND ${GIT_EXECUTABLE} rev-parse --short HEAD
        OUTPUT_VARIABLE GIT_VERSION
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_QUIET
        WORKING_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}
    )
    set(BUILD_VERSION "${BUILD_VERSION}-g${GIT_VERSION}")
else()
    set(BUILD_VERSION "${BUILD_VERSION}")
endif()

# Build configurations
if(ENABLE_ASAN)
    set(BUILD_VERSION "${BUILD_VERSION}-asan")
    list(APPEND BUILD_FLAGS_C -fsanitize=address -fno-omit-frame-pointer)
    list(APPEND LINK_LIBRARIES_C asan)
endif()

if(ENABLE_GCOV)
    set(BUILD_VERSION "${BUILD_VERSION}-gcov")
    list(APPEND BUILD_FLAGS_C -ftest-coverage -fprofile-arcs)
    list(APPEND BUILD_FLAGS_RUST -C instrument-coverage)
    list(APPEND LINK_LIBRARIES_C gcov)
endif()

# Build flags
list(APPEND BUILD_FLAGS_C
    -std=gnu99 -O2 -Wall -Wextra -Werror
    -DBUILD_VERSION="${BUILD_VERSION}" -D_FORTIFY_SOURCE=2
    -Wtrampolines -Wformat=2 -Wstrict-prototypes -Wdate-time
    -Wstack-usage=8192 -Wfloat-equal -Wswitch-default
    -Wshadow -Wconversion -Wcast-qual -Wunused -Wundef
    -funsigned-char -fstack-protector-all -fpic -fpie -ftrapv
    -fstack-check -freg-struct-return -fno-canonical-system-headers
    -fno-common -pipe -fdebug-prefix-map=old=new
)

# The -Werror=cast-align compiler flag causes issues on riscv64 GCC,
# while the same operations do not error on aarch64. This appears to be
# a compiler-specific problem. Temporarily disable this option as a
# workaround since applying fixes would require intrusive code changes
# across multiple files.
if(NOT ARCH STREQUAL "riscv64")
    list(APPEND BUILD_FLAGS_C
        -Wcast-align
    )
endif()

list(APPEND BUILD_FLAGS_RUST
    --cfg unsound_local_offset
    -D warnings
    -C link-arg=-s
    -C strip=symbols
    -C overflow_checks=yes
    -C relocation_model=pic
    -C force-frame-pointers=yes
    -W rust_2021_incompatible_closure_captures
)

# Link flags
list(APPEND LINK_FLAGS_C
    -pie
    -Wl,-z,relro,-z,now
    -Wl,-z,noexecstack -rdynamic
    -Wl,-Bsymbolic
    -Wl,-no-undefined
)

if(CMAKE_BUILD_TYPE STREQUAL "Debug" OR CMAKE_BUILD_TYPE STREQUAL "RelWithDebInfo")
    list(APPEND BUILD_FLAGS_C -g)
elseif(CMAKE_BUILD_TYPE STREQUAL "Release")
    list(APPEND LINK_FLAGS_C -s)
endif()

# Install directories
set(SYSCARE_BINARY_DIR  "${CMAKE_INSTALL_FULL_BINDIR}")
set(SYSCARE_LIBEXEC_DIR "${CMAKE_INSTALL_FULL_LIBEXECDIR}/syscare")
set(SYSCARE_SERVICE_DIR "${CMAKE_INSTALL_PREFIX}/lib/systemd/system")

# Print build info
message("---------------------------------------------------------")
message("███████╗██╗   ██╗███████╗ ██████╗ █████╗ ██████╗ ███████╗")
message("██╔════╝╚██╗ ██╔╝██╔════╝██╔════╝██╔══██╗██╔══██╗██╔════╝")
message("███████╗ ╚████╔╝ ███████╗██║     ███████║██████╔╝█████╗  ")
message("╚════██║  ╚██╔╝  ╚════██║██║     ██╔══██║██╔══██╗██╔══╝  ")
message("███████║   ██║   ███████║╚██████╗██║  ██║██║  ██║███████╗")
message("╚══════╝   ╚═╝   ╚══════╝ ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝")
message("---------------------------------------------------------")
message("-- Build type: ${CMAKE_BUILD_TYPE}")
message("-- Build version: ${BUILD_VERSION}")
message("-- Rust flags: ${BUILD_FLAGS_RUST}")
message("-- Build flags: ${BUILD_FLAGS_C}")
message("-- Link flags: ${LINK_FLAGS_C}")
message("-- Link libraries: ${LINK_LIBRARIES_C}")
message("-- Binary  directory: ${SYSCARE_BINARY_DIR}")
message("-- Libexec directory: ${SYSCARE_LIBEXEC_DIR}")
message("-- Service directory: ${SYSCARE_SERVICE_DIR}")
message("---------------------------------------------------------")

# Apply all flags
add_compile_options(${BUILD_FLAGS_C})
add_link_options(${LINK_FLAGS_C})
link_libraries(${LINK_LIBRARIES_C})

# Build rust executables
set(RUST_TARGET_DIR "${CMAKE_CURRENT_BINARY_DIR}/target")
set(RUST_OUTPUT_DIR "${RUST_TARGET_DIR}/release")
foreach(CURR_FLAG IN LISTS BUILD_FLAGS_RUST)
    set(RUST_FLAGS "${RUST_FLAGS} ${CURR_FLAG}")
endforeach()

add_custom_target(syscare ALL
    COMMAND ${CMAKE_COMMAND} -E env
        "BUILD_VERSION=${BUILD_VERSION}"
        "RUSTFLAGS=${RUST_FLAGS}"
        cargo build --release --target-dir "${RUST_TARGET_DIR}"
    WORKING_DIRECTORY ${CMAKE_CURRENT_LIST_DIR}
)

set_target_properties(syscare PROPERTIES
    ADDITIONAL_CLEAN_FILES "${RUST_TARGET_DIR}"
)

add_custom_target(upatch-helpers ALL
    COMMAND ln -sf upatch-helper upatch-cc
    COMMAND ln -sf upatch-helper upatch-c++
    DEPENDS syscare
    WORKING_DIRECTORY ${RUST_OUTPUT_DIR}
)

# Install rust binaries
install(
    PROGRAMS
        ${RUST_OUTPUT_DIR}/syscare
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_BINARY_DIR}
)

install(
    PROGRAMS
        ${RUST_OUTPUT_DIR}/syscared
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
    DESTINATION
        ${SYSCARE_BINARY_DIR}
)

install(
    PROGRAMS
        ${RUST_OUTPUT_DIR}/syscare-build
        ${RUST_OUTPUT_DIR}/upatch-build
        ${RUST_OUTPUT_DIR}/upatch-helper
        ${RUST_OUTPUT_DIR}/upatch-cc
        ${RUST_OUTPUT_DIR}/upatch-c++
    PERMISSIONS
        OWNER_READ OWNER_WRITE OWNER_EXECUTE
        GROUP_READ GROUP_EXECUTE
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_LIBEXEC_DIR}
)

# Build other components
add_subdirectory(upatch-diff)
add_subdirectory(upatch-manage)
add_subdirectory(misc)
