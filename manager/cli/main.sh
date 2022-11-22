#!/bin/bash
#SPDX-License-Identifier: Mulan-PSL2.0

set -e

PATCHESDIR="/usr/lib/syscare/patches"
PATCH_BUILD_CMD="/usr/libexec/syscare/syscare-build"

function check_patches_dir() {
	local count=`ls $PATCHESDIR|wc -w`

	if [ "$count" > 0 ]; then
		return 0
	fi
	#syscare patches is empty
	return 1
}

function patch_is_syscare() {
	files=$(ls $PATCHESDIR)
	#patch name is exist return 0
	for filename in $files; do
		if [[ "$PATCHNAME" = "$filename" ]]; then
			if check_patches_dir == 0 ; then
				#echo "There is no patch!"
				return 0
			fi
		fi
	done
	echo "$PATCHNAME is not syscare patch!"
 	return 1
}

function check_patch_type() {
	patch_type=`cat $PATCHESDIR/$PATCHNAME/patch_info | grep "type" | awk -F ':' '{print $2}' | xargs echo -n`
	if [ "${patch_type}" == "KernelPatch" ]; then
	#if [${patch_type} == "KernelPatch"]; then
		return 0
	fi
	return 1
}

function check_version() {
	local kv=`uname -r`
	kernel_version="kernel-"${kv%.*}
	patch_version=`cat $PATCHESDIR/$PATCHNAME/patch_info | grep "target" | awk -F ':' '{print $2}' | xargs echo -n`
	if [[ "$kernel_version" != "$patch_version" ]]; then
		echo "patch versio is mismatch patch version"
		return 1
	fi
	return 0
}

function check_patched() {
	lsmod | grep $PATCHNAME > /dev/null

	if [ `echo $?` -eq 0 ]; then
		echo "ok"
		return 0
	fi
	return 1

}

function get_binary() {
	package_name=`cat $PATCHESDIR/$PATCHNAME/patch_info | grep "target" | awk -F ':' '{print $2}' | xargs echo -n`
	echo `rpm -ql $package_name |grep "\/$package_name$"  | xargs file | grep ELF | awk  -F: '{print $1}'`
}

function build_patch() {
	$PATCH_BUILD_CMD $@
}

function apply_patch() {
	patch_is_syscare || return 1
	check_version || return 1
	check_patched && return 0

	if  (check_patch_type 0) ; then
		insmod $PATCHESDIR/$PATCHNAME/$PATCHNAME.ko
		echo "ok"
		return
		#if [echo $? -eq 0]
		#modprobe $PATCHNAME
	else
		upatch-tool apply -b $(get_binary) -p $PATCHESDIR/$PATCHNAME/$PATCHNAME.ko
	fi
}

function remove_patch() {
        patch_is_syscare || return 1
	check_version || return 1

	local patch_file=/sys/kernel/livepatch/$PATCHNAME/enabled
	if check_patch_type 0 ; then
		if [ `cat $patch_file` -eq 1 ]; then
			echo "patch is in use"
	       		return
	 	else
			rmmod $PATCHNAME
			echo "ok"
			return
		fi
	else
		upatch-tool remove -b $(get_binary) -p $PATCHESDIR/$PATCHNAME/$PATCHNAME.ko
	fi
}

function active_patch() {
        patch_is_syscare || return 1
	check_version || return 1

	#判断是否已经是1
	local patch_file=/sys/kernel/livepatch/$PATCHNAME/enabled

	if check_patch_type 0 ; then
		if [ `cat $patch_file` -eq 1 ] ; then
			echo "ok"
			return
		else
			echo 1 > $patch_file
			echo "ok"
			return
		fi
	else
		upatch-tool deactive -b $(get_binary) -p $PATCHESDIR/$PATCHNAME/$PATCHNAME.ko
	fi
}

function deactive_patch() {
        patch_is_syscare || return 1
	check_version || return 1

	local patch_file=/sys/kernel/livepatch/$PATCHNAME/enabled

	if check_patch_type 0 ; then
		if [ `cat $patch_file` -eq 0 ] ; then
			echo "ok"
			return
		else
			echo 0 > $patch_file
			echo "ok"
			return
		fi
	else
		upatch-tool deactive -b $(get_binary) -p $PATCHESDIR/$PATCHNAME/$PATCHNAME.ko
	fi
}

function file_list() {
        if (patch_is_syscare 1) && (check_version 1) ; then
                return
        fi

	echo `cat $PATCHESDIR/$PATCHNAME/patch_info | grep "patch list" | awk -F ':' '{print $2}' | xargs echo -n`
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

	if [ `cat /sys/kernel/livepatch/$PATCHNAME/enabled` -eq 1 ]; then
		echo "$PATCHNAME ACTIVE"
		return
	fi
	echo "$PATCHNAME DEACTIVE"
	return
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
		apply_patch
		;;
	active	|--active-patch)
		PATCHNAME="$2"
		active_patch
		;;
	deactive	|--deactive-patch)
		PATCHNAME="$2"
		deactive_patch
		;;
	remove	|--remove-patch)
		PATCHNAME="$2"
		remove_patch
		;;
	list	|--all-patch)
		patch_list
		;;
	status	|--patch-status)
		PATCHNAME=$2
		patch_status
		;;
	*)
		echo "command not found"
		break
esac
