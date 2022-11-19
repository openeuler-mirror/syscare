#!/bin/bash
#SPDX-License-Identifier: mulan ??

PATCHESDIR="/usr/lib/syscare/patches"

# TODO: auto add rpm funcion
<<EOF
install_rpm() {
	#check kernel version x86??
	local kernel_version =`uname -r`  
	#load patch version
	yum list |grep  ${kernel_version%.*}
	if [echo $? -eq 0]; then 
		yum install ${kernel_version%.*}.src.rpm
	else
		echo "the ${kernel_version%.*}.rpm is not found"
		return
	fi
	#install patch rpm
	rpm -ivh ${kernel_version%.*}.rpm

}

remove_rpm() {
	read -p "Please enter the Y/N:" para
	case $para in
		[yY])
			echo "entered Y"
			dnf erase `rpm -q --whatprovides /usr/lib/syscare/patches/$PATCHNAME`
			;;
		[nN])
			echo "entered N"
			;;
		*)
			echo "Invalid input ..."
		read -p "Please enter any key to exit" exit
		exit 1
esac

	#dnf erase `rpm -q --whatprovides /usr/lib/syscare/patches/$PATCHNAME`
}
EOF

check_patches_dir() {
	local count=`ls $PATCHESDIR|wc -w`
	
	if [ "$count" > 0 ]; then
		return 0
	fi
	#syscare patches is empty
	return 1
}

patch_is_syscare() {
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

check_patch_type() {
	patch_type=`cat $PATCHESDIR/$PATCHNAME/patch_info | grep "type" | awk -F ':' '{print $2}' | xargs echo -n`
	if [ "${patch_type}" == "KernelPatch" ]; then
	#if [${patch_type} == "KernelPatch"]; then
		return 0
	fi
	return 1
}

check_version() {
	local kv=`uname -r`
	kernel_version="kernel-"${kv%.*}
	patch_version=`cat $PATCHESDIR/$PATCHNAME/patch_info | grep "target" | awk -F ':' '{print $2}' | xargs echo -n`
	if [[ "$kernel_version" != "$patch_version" ]]; then
		echo "patch versio is mismatch patch version"
		return 1
	fi
	return 0
}

check_patched() {
	lsmod | grep $PATCHNAME > /dev/null

	if [ `echo $?` -eq 0 ]; then
		echo "ok"
		return 0
	fi
	return 1

}

get_binary() {
	package_name=`cat $PATCHESDIR/$PATCHNAME/patch_info | grep "target" | awk -F ':' '{print $2}' | xargs echo -n`
	echo `rpm -ql $package_name |grep "\/$package_name$"  | xargs file | grep ELF | awk  -F: '{print $1}'`
}

apply_patch() {
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
#EOF
}

remove_patch() {
	#判断模块是否存在??
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

active_patch() {
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

deactive_patch() {
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

file_list() {
        if (patch_is_syscare 1) && (check_version 1) ; then
                return
        fi

	echo `cat $PATCHESDIR/$PATCHNAME/patch_info | grep "patch list" | awk -F ':' '{print $2}' | xargs echo -n`
}

patch_list() {
	files=$(ls $PATCHESDIR)
	if [[ $files = "" ]]; then
		echo "no patch"
		return 1
	fi

	for file in $files;do
		echo "$file"
	done
	
}

patch_status() {
	patch_is_syscare || return 1
	check_version || return 1

	if [ `cat /sys/kernel/livepatch/$PATCHNAME/enabled` -eq 1 ]; then
		echo "$PATCHNAME ACTIVE"
		return
	fi
	echo "$PATCHNAME DEACTIVE"
	return	
}

usage() {
	echo "usage: syscare <command> [<args>]" >&2
	echo "	apply <patch-name>	apply patch into the running kernel or process" >&2
	echo "	active <patch-name>	activate patch into the running kernel or process" >&2
	echo "	deactive <patch-name>	deactive patch" >&2
	echo "	remove <patch-name>	remove the patch in kernel or process" >&2
	echo "	list			query local patched list"
	echo "	-h, --help	show this help message" >&2
}

#while [[ $# -gt 0]];do
if [[ $# -gt 4 ]]; then
	echo "parameter more than 3"
	return 1
fi

while [[ $# -gt 0 ]]; do
	case "$1" in
		help	|--help)
			usage
			exit 0
			;;
		apply	|--apply-patch)
			PATCHNAME="$2"
			apply_patch
			shift
			;;
		active	|--active-patch)
			PATCHNAME="$2"
			active_patch
			shift
			;;
		deactive	|--deactive-patch)
			PATCHNAME="$2"
			deactive_patch
			shift
			;;
		remove	|--remove-patch)
			PATCHNAME="$2"
			remove_patch
			shift
			;;
		list	|--all-patch)
			patch_list
			shift
			;;
		status	|--patch-status)
			PATCHNAME=$2
			patch_status
			shift
			;;
		#TODO: auto add RPM func
		#installrpm |--install-patch-rpm)
		#	install_rpm
		#	shift
		#	;;
		*)
			echo "command not found"
			break
	esac
	shift
done
