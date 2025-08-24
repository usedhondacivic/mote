function alert_reboot() {
    document.getElementById("alert").hidden = false;
    setTimeout(function() {
        location.reload();
    }, 3000);
}
function uid_updated() {
    document.getElementById("uid_submit").hidden = false;
}

function uid_submit() {
    let uid = document.getElementById("uid").value

    if (uid.length > 5) {
        document.getElementById("uid_submit").hidden = true;
        alert_reboot();
    } else {
        alert("Unique identifier must have > 5 characters.");
    }
}

function update_selected_network(name) {
    document.getElementById("selected_network").innerHTML = "Network: " + name;
}

async function fetch_networks() {
    let response = await fetch("./networks.json", {
        cache: 'no-store'
    });
    let networks = await response.json();
    var tableHTML = `
        <tr>
            <th>SSID</th>
            <th>Strength</th>
            <th>Security</th>
        </tr>
    `;
    Object.entries(networks).forEach(([key, value]) => {
        tableHTML += `<tr onclick="update_selected_network('` + key + `')">`;
        tableHTML += "<td>" + key + "</td>";
        tableHTML += "<td>" + value + "</td>";
        tableHTML += "</tr>";
    })

    document.getElementById("wifi_table").innerHTML = tableHTML;
}

let bit_to_emoji = {
    "pass": "✅",
    "warning": "⚠️",
    "fail": "❌"
}

async function fetch_bit() {
    let response = await fetch("./bit.json", {
        cache: 'no-store'
    });
    let bit = await response.json();
    var tableHTML = `
        <tr>
            <th>Test</th>
            <th>Status</th>
        </tr>
    `;
    Object.entries(bit).forEach(([key, value]) => {

        tableHTML += "<tr>";
        tableHTML += "<td>" + key + "</td>";
        tableHTML += "<td>" + bit_to_emoji[value] + "</td>";
        tableHTML += "</tr>";
    })

    document.getElementById("bit_table").innerHTML = tableHTML;
}

window.onload = function() {
    // Fetch available networks on page load
    fetch_networks();
    // Fetch BIT results on page load
    fetch_bit();
};

async function test_webusb() {
    const device = await navigator.usb.requestDevice({ filters: [{ vendorId: 0xf569 }] });
    await device.open();
    await device.claimInterface(1);
    device.transferIn(1, 64).then(data => console.log(data));
    await device.transferOut(1, new Uint8Array([1, 2, 3]));
}
