# corsairmi-mqtt

This is a quick and dirty daemon to read power measurements from my power supply, and publish them to a MQTT server.

This code is published for reference only; there are hard coded values that you will not want to use.

## Usage Reference

1. `cargo install --path .`
2. `cp corsairmi.service /lib/systemd/system/corsairmi.service`
3. Reload systemd `sudo systemctl daemon-reload`
4. Enable autostart `sudo systemctl enable corsairmi.service`
5. Check journal if there are problems `sudo journalctl -xe`

Other commands:

* Stop: `sudo service corsairmi start`
* Status: `service corsairmi status`
* Start: `sudo service corsairmi start`
* Restart: `sudo service corsairmi restart`
