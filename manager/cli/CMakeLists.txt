# SPDX-License-Identifier: MulanPSL2

cmake_minimum_required(VERSION 3.14)
project(manager-cli)

set(MANAGER_CLI "syscare")
set(MANAGER_CLI_INSTALL_DIR /usr/bin)

add_custom_target(manager-cli ALL
        COMMENT "Compiling manager-cli ..."
        BYPRODUCTS ${MANAGER_CLI}
        COMMAND cp main.sh syscare
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

install(FILES ${MANAGER_CLI} PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ GROUP_EXECUTE GROUP_READ WORLD_READ WORLD_EXECUTE DESTINATION ${MANAGER_CLI_INSTALL_DIR})
