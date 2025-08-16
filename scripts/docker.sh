#!/usr/bin/env bash

# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Copyright © 2025 RemasteredArch
#
# This file is part of 1N4.
#
# 1N4 is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public
# License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later
# version.
#
# 1N4 is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License along with 1N4. If not, see
# <https://www.gnu.org/licenses/>.

set -euo pipefail

source_dir="$(dirname "$(realpath "$0")")"
# Assumes this is `./scripts/`, relative from the actual project root.
project_root="$(dirname "$source_dir")"

# shellcheck disable=1091
. "$source_dir/utilities.sh"

declare -r script_name='docker'
declare -r script_version='0.1.0'

container_tag='ArchitecturalDogSoftware/ina'
declare -a maybe_sudo=('sudo')

function print_help() {
    print_help_header "$script_name" "$script_version" '[arguments]'

    print_help_argument 'h' 'Displays this screen.'
    print_help_argument 'V' 'Displays the current script version.'

    print_help_argument 's' 'Don'"'"'t use sudo for Docker CLI commands.'
    print_help_argument 't' 'Sets the tag of the Docker images produced.' '[tag]'
}

function print_build_help() {
    print_help_header "$script_name" "$script_version" 'build [arguments]'

    print_help_argument 'h' 'Displays this screen.'
    print_help_argument \
        '-platform' \
        'The Docker platform specifier to target and the Rust CPU architecture to target, e.g., linux/amd64:x86_64' \
        '[docker target]:[rust arch]'
}

function print_export_help() {
    print_help_header "$script_name" "$script_version" 'export [arguments]'

    print_help_argument 'h' 'Displays this screen.'
    print_help_argument \
        '-platform' \
        'The Docker platform specifier to target and, optionally and ignored, the Rust CPU architecture to target, e.g., linux/amd64:x86_64 or linux/amd64' \
        '[docker target]:[rust arch]'
}

function print_run_help() {
    print_help_header "$script_name" "$script_version" 'run [arguments] -- [container arguments]'

    print_help_argument 'h' 'Displays this screen.'
    print_help_argument '-no-cd' 'Running in the current directory instead of the project root.' \
        print_help_argument \
        '-platform' \
        'The Docker platform specifier to run and, optionally and ignored, the Rust CPU architecture to target, e.g., linux/amd64:x86_64 or linux/amd64' \
        '[docker target]:[rust arch]'
}

# Detect the [platfor specifier](https://github.com/containerd/containerd/blob/v1.4.3/platforms/platforms.go#L63) of
# the host.
#
# Takes no arguments, returns a value like `linux/amd64` (NOT `x86_64`) or `linux/arm/v6`. Assumes Docker is installed,
# and (if `-s` isn't used) the user has `sudo` permissions.
function detect_platform() {
    # I _think_ this template includes architecture revisions, e.g. `v6` in `linux/arm/v6`, or at least that they're not
    # available here. I only see `Os` and `Arch` in the source code for Moby (used by Docker CE):
    # <https://github.com/moby/moby/blob/feeaa16/api/types/types.go#L41-L59>.
    "${maybe_sudo[@]}" docker version --format '{{ .Server.Os }}/{{ .Server.Arch }}'
}

function build() {
    local host_platform
    host_platform="$(detect_platform)"

    local target_docker_platform=''
    local target_rust_arch=''
    local platform_args=()

    while [ -n "${1:-}" ]; do
        case "$1" in
            '-h')
                print_build_help
                exit 0
                ;;

            '--platform')
                target_docker_platform="${2%:*}"
                target_rust_arch="${2//*:/}"
                shift
                ;;

            *)
                echo todo
                exit 1
                ;;
        esac

        shift
    done

    if [ "$target_docker_platform" != '' ] && [ "$target_docker_platform" != "$host_platform" ]; then
        platform_args=('--platform' "$target_docker_platform" '--build-arg' "TARGET_ARCH_RUST=$target_rust_arch")
    fi

    "${maybe_sudo[@]}" docker buildx build --tag "$container_tag" \
        "${platform_args[@]}" \
        "$project_root"
}

function export_tar() {
    local host_platform
    host_platform="$(detect_platform)"

    local target_docker_platform=''
    local platform_args=()
    local tarball_path=''
    local output_args=()

    while [ -n "${1:-}" ]; do
        case "$1" in
            '-h')
                print_export_help
                exit 0
                ;;

            '--platform')
                target_docker_platform="${2%:*}"
                shift
                ;;

            '--output')
                tarball_path="$2"
                output_args=('--output' "$tarball_path")
                shift
                ;;

            *)
                print_export_help
                exit 1
                ;;
        esac

        shift
    done

    if [ "$target_docker_platform" != '' ] && [ "$target_docker_platform" != "$host_platform" ]; then
        platform_args=('--platform' "$target_docker_platform")
    fi

    "${maybe_sudo[@]}" docker image save \
        "${platform_args[@]}" \
        "${output_args[@]}" \
        "$container_tag"

    if [ "$tarball_path" != '' ]; then
        "${maybe_sudo[@]}" chown "$(id -u):$(id -g)" "$tarball_path"
    fi
}

function run() {
    local host_platform
    host_platform="$(detect_platform)"

    local target_docker_platform=''
    local platform_args=()

    local do_cd=1

    while [ -n "${1:-}" ] && [ "$1" != '--' ]; do
        case "$1" in
            '-h')
                print_run_help
                exit 0
                ;;

            '--no-cd')
                do_cd=0
                ;;

            '--platform')
                target_docker_platform="${2%:*}"
                shift
                ;;

            *)
                print_run_help
                exit 1
                ;;
        esac

        shift
    done
    if [ "${1:-}" = '--' ]; then
        shift
    fi

    if [ "$target_docker_platform" != '' ] && [ "$target_docker_platform" != "$host_platform" ]; then
        platform_args=('--platform' "$target_docker_platform")
    fi

    if [ $do_cd = 1 ]; then
        cd "$project_root"
    fi

    # Grant ownership of the necessary directories to the container user.
    sudo chown -R '10001:10001' './log' './res/data'

    # Even if the `docker run` fails, we still need to capture the exit code so that we can still reset ownership.
    local exit_code=0

    # Start the Docker container and attach the TTY.
    "${maybe_sudo[@]}" docker run --tty --rm \
        "${platform_args[@]}" \
        --env-file '.env' \
        --mount 'type=bind,source=./log,target=/app/log' \
        --mount 'type=bind,source=./res,target=/app/res,readonly' \
        --mount 'type=bind,source=./res/data,target=/app/res/data' \
        'ArchitecturalDogSoftware/ina' "$@" ||
        exit_code=$?

    # Reset ownership of the necessary directories back to the current user.
    sudo chown -R "$(id -u):$(id -g)" './log' './res/data' || exit_code=$?

    exit $exit_code
}

while getopts 'hVst:' argument; do
    case "$argument" in
        'h')
            print_help
            exit 0
            ;;
        'V')
            print_version "$script_name" "$script_version"
            exit 0
            ;;

        's') maybe_sudo=() ;;

        't') container_tag="$OPTARG" ;;

        *)
            print_help
            exit 1
            ;;
    esac
done
# Clear arguments from parameters.
shift $((OPTIND - 1))

command="${1:-}"
shift
case "$command" in
    'help')
        print_help
        exit 0
        ;;

    'build')
        build "$@"
        ;;

    'export')
        export_tar "$@"
        ;;

    'run')
        run "$@"
        ;;

    *)
        print_help
        exit 1
        ;;
esac
