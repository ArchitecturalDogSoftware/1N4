#!/usr/bin/env bash

# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Copyright © 2024 Jaxydog
#
# This file is part of 1N4.
#
# 1N4 is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
#
# 1N4 is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License along with 1N4. If not, see <https://www.gnu.org/licenses/>.

declare -r script_name='publish'
declare -r script_version='0.1.0'

function print_version() {
    echo "$script_name v$script_version"
}

function print_argument() {
    local flag="-$1"
    local desc="$2"
    local args

    if [ -n "$3" ]; then
        args=" [$3]\t"
    else
        args="\t\t"
    fi

    echo -e "$flag$args$desc"
}

function print_help() {
    echo "$script_name v$script_version"
    echo -e "usage: $0 [arguments]\n"

    print_argument 'h' 'Displays this screen.'
    print_argument 'v' 'Displays the current script version.'
    print_argument 'c' 'Cleans the build directory before compiling'
    print_argument 'C' 'Cleans the Cargo cache before compiling'
    print_argument 't' 'Override the target triple appended to the finished binary name.'
}

function cancel() {
    declare -i code

    if [[ "$1" =~ ^-?[0-9]+$ ]]; then
        code=$1
        
        shift 1
    else
        code=1
    fi

    if [ $code -eq 0 ]; then
        echo "$*"
    else
        echo "$*" >&2
    fi

    exit $code
}

function or_cancel() {
    cancel $? "$*"
}

function step() {
    local name="$1"
    local command="$2"

    echo -n "- $name..."

    local output

    output="$(eval "$command" 2>&1)"
    
    local code="$?"

    [ $code -ne 0 ] && {
        echo ' Failed'
        echo "$output"
        exit $code
    }

    echo ' Done'
}

declare binary_dir="$PWD/bin"
declare target_dir="$PWD/target/release"
declare executable="$target_dir/ina"
declare checksums="$binary_dir/checksums"
declare target_triple executable_version
declare -i clean_build clean_cache

[ -d "$PWD"/.git ] || {
    cancel "This script must be run within the project's root directory"
}

while getopts 'hvcCt:' argument; do
    case $argument in
        h) print_help; exit 0;;
        v) print_version; exit 0;;
        t) target_triple="$OPTARG";;
        c) clean_build=1;;
        C) clean_cache=1;;
        *) print_help; exit 1;;
    esac
done

echo -e 'Publishing executable\n'

[ -z "$target_triple" ] && {
    target_triple="$(rustup target list | grep 'installed')"
    target_triple="${target_triple%' (installed)'}"
}

[ $clean_build ] && {
    step 'Cleaning build directory' 'cargo clean'
}
[ $clean_cache ] && {
    step 'Cleaning Cargo cache' 'cargo cache -r all'
}

unset clean_build clean_cache

step 'Compiling executable' 'cargo build --release'

[ -d "$binary_dir" ] || {
    step 'Creating binary directory' "mkdir -p '$binary_dir' || or_cancel 'Failed to create binary directory'"
}
[ -z "$(ls -A "$binary_dir")" ] || {
    step 'Clearing binary directory' "rm --interactive=once '$binary_dir'/*"
}

executable_version="$(eval "$executable -V" | sed 's/ina //')"

step 'Copying executable' "cp '$executable' '$binary_dir/ina-$executable_version+$target_triple'"

unset executable executable_version target_triple

escaped_binary_dir="$(echo "$binary_dir" | sed 's/\//\\\//g')"

step 'Generating checksums' "sha256sum '$binary_dir'/* | sed \"s/$escaped_binary_dir\///\" > '$checksums'"

unset escaped_binary_dir

echo -ne '\nChecksums'

if [ -n "$(which clip.exe)" ]; then
    clip.exe < "$checksums"
    echo -n ' (also copied to your clipboard)'
elif [ -n "$(which xclip)" ]; then
    xclip -selection clipboard < "$checksums"
    echo -n ' (also copied to your clipboard)'
fi

echo -e ":\n"
cat "$checksums"
echo -e "\nFiles located within: '${binary_dir/"$PWD"/'.'}'"
