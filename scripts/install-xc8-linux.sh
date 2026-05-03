#!/usr/bin/env bash
set -euo pipefail

xc8_version="${XC8_VERSION:-3.10}"
xc8_installer="xc8-v${xc8_version}-full-install-linux-x64-installer.run"
xc8_url="${XC8_URL:-https://ww1.microchip.com/downloads/aemDocuments/documents/DEV/ProductDocuments/SoftwareTools/${xc8_installer}}"
xc8_home="${XC8_HOME:-$HOME/.local/microchip/xc8/v${xc8_version}}"

packs_home="${MICROCHIP_PACKS_HOME:-$HOME/.mchp_packs/Microchip}"
download_dir="${RUNNER_TEMP:-/tmp}/atu10-toolchain"

install_dfp() {
    local pack="$1"
    local version="$2"
    local sha256="$3"
    local archive="${download_dir}/Microchip.${pack}.${version}.atpack"
    local target="${packs_home}/${pack}/${version}"

    if [[ -d "${target}/xc8" ]]; then
        echo "${pack} ${version} already installed"
        return
    fi

    mkdir -p "${target}"
    curl --fail --location --retry 3 --output "${archive}" \
        "https://packs.download.microchip.com/Microchip.${pack}.${version}.atpack"
    echo "${sha256}  ${archive}" | sha256sum --check -
    unzip -q "${archive}" -d "${target}"
}

mkdir -p "${download_dir}" "${packs_home}"

if [[ ! -x "${xc8_home}/bin/xc8-cc" ]]; then
    curl --fail --location --retry 3 --output "${download_dir}/${xc8_installer}" "${xc8_url}"
    chmod +x "${download_dir}/${xc8_installer}"
    "${download_dir}/${xc8_installer}" \
        --installer-language en \
        --mode unattended \
        --unattendedmodeui none \
        --installerfunction installcompiler \
        --LicenseType FreeMode \
        --ModifyAll 0 \
        --netservername "" \
        --prefix "${xc8_home}"
fi

if [[ ! -x "${xc8_home}/bin/xc8-cc" ]]; then
    echo "XC8 installer completed, but ${xc8_home}/bin/xc8-cc was not found" >&2
    exit 1
fi

install_dfp "PIC12-16F1xxx_DFP" "1.9.258" \
    "fe8faaac84224a139891a0f3cc206dda81fb97287d06ed5706c81a687e0e0821"
install_dfp "PIC16F1xxxx_DFP" "1.31.465" \
    "4ab50366f9ab7609c14c1b93e65cd9222d15d62d29fb2252e3ea0b9255dc125f"

echo "${xc8_home}/bin" >> "${GITHUB_PATH:-/dev/null}"
echo "XC8 installed at ${xc8_home}"
