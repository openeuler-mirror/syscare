#!/bin/bash
#SPDX-License-Identifier: Mulan-PSL2.0

set -e

PATCHESDIR="/usr/lib/syscare/patches"
PATCH_BUILD_CMD="/usr/libexec/syscare/syscare-build"
UPATCH_TOOL="/usr/libexec/syscare/upatch-tool"
PATCH_TYPE="kernel"
ELF_PATH=""
PATCHNAME=""

function check_patches_dir() {
	local count=$(ls $PATCHESDIR|wc -w)

	if [ "$count" -gt 0 ]; then
		return 0
	fi
	#syscare patches is empty
	return 1
}

function patch_is_syscare() {
	files=$(ls $PATCHESDIR)
	#patch name is exist return 0
	for filename in $files; do
		if [ "$PATCHNAME" == "$filename" ]; then
			if check_patches_dir == 0 ; then
				#echo "There is no patch!"
				return 0
			fi
		fi
	done
	echo "$PATCHNAME is not syscare patch!"
 	return 1
}

function get_patch_type() {
	local patch_type=$(cat $PATCHESDIR/$PATCHNAME/patch_info | grep "type" | awk -F ':' '{print $2}' | xargs echo -n)
	if [ "${patch_type}" == "KernelPatch" ]; then
		PATCH_TYPE="kernel"
	else
		PATCH_TYPE="user"
	fi
}

function get_binary() {
	[ "$PATCH_TYPE" == "kernel" ] && return

	local package_name=$(cat $PATCHESDIR/$PATCHNAME/patch_info | grep "target" | awk -F ':' '{print $2}' | xargs echo -n)
	local binary_name=$(cat $PATCHESDIR/$PATCHNAME/patch_info | grep "elf_name" | awk -F ':' '{print $2}' | xargs echo -n)
	ELF_PATH=$(rpm -ql $package_name |grep "\/$binary_name$"  | xargs file | grep ELF | awk  -F: '{print $1}')
}

function prepare_patchinfo {
	get_patch_type
	get_binary
}

function check_version() {
	[ "$PATCH_TYPE" == "kernel" ] || return 0

	local kv=$(uname -r)
	local kernel_version="kernel-"${kv%.*}
	local patch_version=$(cat $PATCHESDIR/$PATCHNAME/patch_info | grep "target" | awk -F ':' '{print $2}' | xargs echo -n)
	if [ "$kernel_version" != "$patch_version" ]; then
		echo "Patch version mismatches with patch version."
		return 1
	fi

	return 0
}

function check_patched() {
	lsmod | grep $PATCHNAME > /dev/null

	if [ `echo $?` -eq 0 ]; then
		return 0
	fi
	return 1

}

function build_patch() {
	$PATCH_BUILD_CMD $@
}

function apply_patch() {
	patch_is_syscare || return 1
#	check_version || return 1
#	check_patched && return 0

	if  [ "$PATCH_TYPE" == "kernel" ] ; then
		insmod $PATCHESDIR/$PATCHNAME/$PATCHNAME.ko
		return
		#if [echo $? -eq 0]
		#modprobe $PATCHNAME
	else
		${UPATCH_TOOL} apply -b "$ELF_PATH" -p $PATCHESDIR/$PATCHNAME/$PATCHNAME
	fi
}

function remove_patch() {
        patch_is_syscare || return 1
	check_version || return 1

	local patch_file=/sys/kernel/livepatch/$PATCHNAME/enabled
	if [ "$PATCH_TYPE" == "kernel" ] ; then
		if [ $(cat $patch_file) -eq 1 ]; then
			echo "patch is in use"
	       		return
	 	else
			rmmod $PATCHNAME
			return
		fi
	else
		${UPATCH_TOOL} remove -b "$ELF_PATH"
	fi
}

function active_patch() {
        patch_is_syscare || return 1
	check_version || return 1

	#判断是否已经是1
	local patch_file=/sys/kernel/livepatch/$PATCHNAME/enabled

	if [ "$PATCH_TYPE" == "kernel" ] ; then
		if [ $(cat $patch_file) -eq 1 ] ; then
			return
		else
			echo 1 > $patch_file
			return
		fi
	else
		${UPATCH_TOOL} active -b "$ELF_PATH"
	fi
}

function deactive_patch() {
        patch_is_syscare || return 1
	check_version || return 1

	local patch_file=/sys/kernel/livepatch/$PATCHNAME/enabled

	if [ "$PATCH_TYPE" == "kernel" ] ; then
		if [ $(cat $patch_file) -eq 0 ] ; then
			return
		else
			echo 0 > $patch_file
			return
		fi
	else
		${UPATCH_TOOL} deactive -b "$ELF_PATH"
	fi
}

function file_list() {
        if (patch_is_syscare 1) && (check_version 1) ; then
                return
        fi

	echo $(cat $PATCHESDIR/$PATCHNAME/patch_info | grep "patch list" | awk -F ':' '{print $2}' | xargs echo -n)
}

function patch_list() {
	files=$(ls $PATCHESDIR)
	if [[ $files = "" ]]; then
		echo "no patch"
		return 1
	fi

	for file in $files;do
		echo "$file"
	done
}

function patch_status() {
	patch_is_syscare || return 1
	check_version || return 1

	if [ "$PATCH_TYPE" == "kernel" ]; then
		local kernel_state_file="/sys/kernel/livepatch/$PATCHNAME/enabled"

		if [ ! -f "${kernel_state_file}" ]; then
			echo "$PATCHNAME DEACTIVE"
			return
		fi

		if [ $(cat "$kernel_state_file") -eq 1 ]; then
			echo "$PATCHNAME ACTIVE"
			return
		fi
		echo "$PATCHNAME DEACTIVE"
		return
	fi

	echo "Upatch is on processing.."

	#TODO: add user patch
}

function usage() {
	echo -e "\033[1;4mUsage:\033[0m \033[1msyscare\033[0m <command> [<args>]" >&2
	echo "  "
	echo -e "\033[1;4mCommand:\033[0m"
	echo -e "  \033[1mapply\033[0m <patch-name>              Apply patch into the running kernel or process" >&2
	echo -e "  \033[1mactive\033[0m <patch-name>             Activate patch into the running kernel or process" >&2
	echo -e "  \033[1mdeactive\033[0m <patch-name>           Deactive patch" >&2
	echo -e "  \033[1mremove\033[0m <patch-name>             Remove the patch in kernel or process" >&2
	echo -e "  \033[1mlist\033[0m                            Query local patched list"
	echo -e "  \033[1m-h, --help\033[0m                      Show this help message" >&2
	echo "  "
	echo -e "  \033[1mbuild\033[0m                           Build patch, more details:"
	$PATCH_BUILD_CMD --help
}

if [[ $# -lt 1 ]]; then
	echo "need parameters"
	usage
	exit 1
fi

case "$1" in
	help	|-h	|--help)
		usage
		exit 0
		;;
	build	|--build-patch)
		shift
		build_patch $@
		;;
	apply	|--apply-patch)
		PATCHNAME="$2"
		prepare_patchinfo
		apply_patch
		;;
	active	|--active-patch)
		PATCHNAME="$2"
		prepare_patchinfo
		active_patch
		;;
	deactive	|--deactive-patch)
		PATCHNAME="$2"
		prepare_patchinfo
		deactive_patch
		;;
	remove	|--remove-patch)
		prepare_patchinfo
		PATCHNAME="$2"
		remove_patch
		;;
	list	|--all-patch)
		patch_list
		;;
	status	|--patch-status)
		prepare_patchinfo
		PATCHNAME=$2
		patch_status
		;;
	*)
		echo "command not found, use --help to get usage."
		break
esac
