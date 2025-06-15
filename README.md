# UAVSAR

## About
UAVSAR is a prototype flight planning tool for unmanned aerial vehicles (UAVs) designed to support search and rescue (SAR) operations. Given a polygonal area and drone camera specifications (field of view, altitude, desired image overlap), it generates a zigzag flight path ensuring full photographic coverage of the area. The application is implemented as a cross-platform desktop app using Tauri and integrates mapping capabilities via MapLibre.

The application is designed with usability and learnability as it's core tenant, to support SAR volunteers in using the tool in SAR operations without extensive training.

## How to Get Started
1. Install the required dependecies for the project. These include the Rust toolchain, npm and the Tauri CLI. Instructions to install these tools can be found online.

2. Clone the repository
```sh
git clone https://github.com/slammywill/UAVSAR.git
```

3. Enter into the repository directory
```sh
cd UAVSAR
```

4. Install npm modules
```sh
npm install
```

5. Run the application
```sh
npm run tauri dev
```

## How to Run
Once the repository has been installed and built, the app can easily be run again with step 5 of the 'How to Get Started' section. Alternatively, the application can be built into an application that will run on Mac, Windows and Linux as a typical application. To build the application, run the following command:
```sh
npm run tauri build
```


### Author
Sam Willems 