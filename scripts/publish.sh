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
declare target_dir="$PWD/target/release"
declare executable="$target_dir/ina"
declare -r checksums="$binary_dir/checksums"
declare target_triple executable_version
declare -i clean_build clean_cache super_optimized

function print_help() {
    print_help_header "$script_name" "$script_version" '[arguments]'
    print_help_argument 'h' 'Displays this screen.'
    print_help_argument 'v' 'Displays the current script version.'
    print_help_argument 'c' 'Cleans the build directory before compiling'
    print_help_argument 'C' 'Cleans the Cargo cache before compiling'
    print_help_argument 't' 'Override the target triple appended to the finished binary name.'
    print_help_argument 'p' 'Use a build profile that optimizes for performance'
}

if [ ! -d "$PWD"/.git ]; then 
    cancel_execution "This script must be run within the project's root directory"
fi

while getopts 'hvcCt:p' argument; do
    case $argument in
        h) print_help; exit 0;;
        v) print_version "$script_name" "$script_version"; exit 0;;
        t) target_triple="$OPTARG";;
        c) clean_build=1;;
        C) clean_cache=1;;
        p) super_optimized=1;;
        *) print_help; exit 1;;
    esac
done

echo -e 'Publishing executable\n'

if [ -z "$target_triple" ]; then
    target_triple="$(rustup target list | grep 'installed')"
    target_triple="${target_triple%' (installed)'}"
fi

if [ $clean_build ]; then
    eval_step 'Cleaning build directory' 'cargo clean'
fi

unset clean_build

if [ $clean_cache ]; then
    eval_step 'Cleaning Cargo cache' 'cargo cache -r all'
fi

unset clean_cache

if [ $super_optimized ]; then
    eval_step 'Compiling optimized executable' 'cargo build --profile=release-super-optimized'

    target_dir="$PWD/target/release-super-optimized"
    executable="$target_dir/ina"
else
    eval_step 'Compiling executable' 'cargo build --release'
fi

unset super_optimized

if [ ! -d "$binary_dir" ]; then
    eval_step 'Creating binary directory' "mkdir -p '$binary_dir' || or_cancel_execution 'Failed to create binary directory'"
fi
if [ -n "$(ls -A "$binary_dir")" ]; then
    eval_step 'Clearing binary directory' "rm --interactive=once '$binary_dir'/*"
fi

executable_version="$(eval "$executable -V" | sed 's/ina //')"

eval_step 'Copying executable' "cp '$executable' '$binary_dir/ina-$executable_version+$target_triple'"

unset executable_version target_triple

escaped_binary_dir="$(echo "$binary_dir" | sed 's/\//\\\//g')"

eval_step 'Generating checksums' "sha256sum '$binary_dir'/* | sed \"s/$escaped_binary_dir\///\" > '$checksums'"

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
