# Install service
install(
    FILES
        ${CMAKE_CURRENT_LIST_DIR}/upatch.service
        ${CMAKE_CURRENT_LIST_DIR}/syscare.service
    PERMISSIONS
        OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ
        WORLD_READ WORLD_EXECUTE
    DESTINATION
        ${SYSCARE_SERVICE_DIR}
)
