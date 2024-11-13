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

# Usage:
# print_help_header 'script_name' '0.1.0' '[arguments] [input]'
print_help_header() {
    local name="$1"
    local version="$2"
    local usage="$3"

    echo -e "$name v$version\n"
    echo -e "usage: $name $usage\n"
}

# Usage:
# print_help_argument 'h' 'Prints a help menu.'
# print_help_argument 'o' 'Sets the output file.' 'FILE'
print_help_argument() {
    local flag="-$1"
    local description="$2"
    local arguments

    if [ -n "$3" ]; then
        arguments=" [$3]\t"
    else
        arguments="\t\t"
    fi

    echo -e "$flag$arguments$description"
}

# Usage:
# print_version 'script_name' '0.1.0'
print_version() {
    local name="$1"
    local version="$2"

    echo "$name v$version"
}

# Usage:
# eval_step 'Clean build directory' 'cargo clean'
eval_step() {
    local text="$1"
    local command="$2"

    echo -n "- $text..."

    local output

    # Capture the command's output from both stdout and stderr.
    output="$(eval "$command" 2>&1)"

    local code=$?

    echo -n ' '

    # Exit early, printing the command output if it failed.
    if [ $code -ne 0 ]; then
        echo -e "Failed!\n$output"
        exit $code
    else
        echo 'Done!'
    fi
}

# Usage:
# cancel_execution 0 'Exited successfully'
# cancel_execution 2 'Failed execution with a code of 2'
# cancel_execution 'Failed execution' # Will use an exit code of 1
cancel_execution() {
    declare -i code

    if [[ "$1" =~ ^-?[0-9]+$ ]]; then
        code=$1
        shift 1
    else
        code=1
    fi

    declare -i stream

    if [ $code -eq 0 ]; then
        stream=1
    else
        stream=2
    fi

    echo -e "$*" >&$stream

    exit $code
}

# Usage:
# echo 'Hello, world!' || or_cancel_execution 'Failed to echo'
or_cancel_execution() {
    # Directly uses the return code of the previous command
    cancel_execution $? "$*"
}
