#!/bin/bash

RECORD_FILE="/usr/lib/syscare/patch-record"
LIBSYSCARE_DIR="/usr/lib/syscare"
SYSCARE_BIN="/usr/bin/syscare"

if [ ! -e "${RECORD_FILE}" ]; then
	mkdir -p "${LIBSYSCARE_DIR}"
	touch "${RECORD_FILE}"
fi

while read line
do
	patch=$(echo "${line}" | awk '{print $1}' | awk -F: '{print $2}')
	isactive=$(echo "${line}" | awk '{print $2}' | awk -F: '{print $2}')
	if [ "${isactive}" == "1" ]; then
		"${SYSCARE_BIN}" apply "${patch}"
	fi
done < "${RECORD_FILE}"
