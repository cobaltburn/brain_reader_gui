# brain_reader_gui

The brain_reader_gui App is a graphical user interface application written in Rust using the Iced library. It serves as the front-end component of the brain_server, allowing users to control a drone using brain wave readings. The app communicates with the brain_server, sends brain wave readings from a connected helmet, and receives interpreted movements to control the drone.

## Features

- Intuitive and user-friendly graphical interface built with the Iced library
- Establishes a connection with the brain_server to send brain wave readings and receive interpreted movements
- Supports connection to a compatible brain wave reading helmet
- Sends commands to the drone based on the received movements
- Provides real-time feedback and visualization of brain wave readings and drone control
- Cross-platform compatibility (Windows, macOS, Linux)
