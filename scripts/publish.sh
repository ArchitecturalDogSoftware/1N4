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

source "$(dirname "$0")/utilities.sh"

declare -r script_name='publish'
declare -r script_version='0.3.0'

declare -r binary_dir="$PWD/bin"
declare -r target_dir="$PWD/target"

declare -i clean_bin clean_build clean_cache
declare build_profile='release'
declare build_target

function print_help() {
    print_help_header "$script_name" "$script_version" '[arguments]'
    
    print_help_argument 'h' 'Displays this screen.'
    print_help_argument 'V' 'Displays the current script version.'
    
    print_help_argument 'c' 'Cleans the build directory before compiling'
    print_help_argument 'C' 'Cleans the Cargo cache before compiling'
    print_help_argument 'r' 'Cleans previously compiled executables'
    
    print_help_argument 't' "Set the build target triple (defaults to '$build_target')"
    print_help_argument 'p' "Set the build profile (defaults to '$build_profile')"
}

if [ ! -d "$PWD"/.git ]; then 
    cancel_execution "This script must be run within the project's root directory"
fi

build_target="$(rustup target list | grep 'installed')"
build_target="${build_target%' (installed)'}"

while getopts 'hVcCrt:p:' argument; do
    case "$argument" in
        'h') print_help; exit 0;;
        'V') print_version "$script_name" "$script_version"; exit 0;;

        'c') clean_build=1;;
        'C') clean_cache=1;;
        'r') clean_bin=1;;

        't') build_target="$OPTARG";;
        'p') build_profile="$OPTARG";;

        *) print_help; exit 1;;
    esac
done

if [ "$build_profile" = 'dev' ]; then
    declare -r executable_path="$target_dir/$build_target/debug/ina"
else
    declare -r executable_path="$target_dir/$build_target/$build_profile/ina"
fi
declare -r checksum_path="$binary_dir/checksums"

echo -e 'Publishing executable\n'

if [ -z "$build_profile" ]; then
    build_profile='release'
fi
if [ -z "$build_target" ]; then
    build_target="$(rustup target list | grep 'installed')"
    build_target="${build_target%' (installed)'}"
fi

if [ $clean_build ]; then
    eval_step 'Cleaning build directory' 'cargo clean'
fi

unset clean_build

if [ $clean_cache ]; then
    eval_step 'Cleaning Cargo cache' 'cargo cache -r all'
fi

unset clean_cache

if [ $clean_bin ]; then
    eval_step 'Cleaning previous builds' "[ -e '$binary_dir' ] && rm -r '$binary_dir'"
fi

eval_step 'Compiling executable' "cargo build --profile='$build_profile' --target='$build_target'"

executable_version="$(eval "$executable_path -V" | sed 's/ina //')"
output_path="$binary_dir/ina-$executable_version+$build_profile.$build_target"

if [ ! -d "$binary_dir" ]; then
    eval_step 'Creating binary directory' "mkdir -p '$binary_dir' || or_cancel_execution 'Failed to create binary directory'"
fi
if [ -e "$output_path" ]; then
    eval_step 'Removing previous binary' "rm '$output_path'"
fi

eval_step 'Copying executable' "cp '$executable_path' '$output_path'"

unset build_profile build_target executable_version output_path

escaped_binary_dir="$(echo "$binary_dir" | sed 's/\//\\\//g')"

if [ -e "$checksum_path" ]; then
    rm --interactive=once "$checksum_path"
fi

eval_step 'Generating checksums' "sha256sum '$binary_dir'/* | sed \"s/$escaped_binary_dir\///\" > '$checksum_path'"

unset escaped_binary_dir

echo -ne '\nChecksums'

if [ -n "$(which clip.exe)" ]; then
    clip.exe < "$checksum_path"
    echo -n ' (also copied to your clipboard)'
elif [ -n "$(which xclip)" ]; then
    xclip -selection clipboard < "$checksum_path"
    echo -n ' (also copied to your clipboard)'
fi

echo -e ":\n"
cat "$checksum_path"
echo -e "\nFiles located within: '${binary_dir/"$PWD"/'.'}'"
