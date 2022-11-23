#!/bin/bash

PATCHESDIR="/usr/lib/syscare/patches"
RECORD_FILE="/usr/lib/syscare/patch-record"

if [ ! -e ${RECORD_FILE} ]; then
    touch ${RECORD_FILE}
fi

while read line
do
    patch=$(echo $line | awk '{print $1}' | awk -F: '{print $2}')
    isactive=$(echo $line | awk '{print $2}' | awk -F: '{print $2}')
    if [ ${isactive} == '1' ]
    then
        syscare apply ${patch}
    fi
done < ${RECORD_FILE}