use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    writer::Writer,
};
use zip::{write::FileOptions, write::ZipWriter, CompressionMethod::Stored};

use crate::flight_path::{Drone, Waypoint};
use std::{fs, io::Cursor, io::Write};

pub async fn write_wqml(waypoints: &[Waypoint], heading_angle: &f64, drone: &Drone) {
    match create_kmz(waypoints, heading_angle, drone).await {
        Ok(_) => println!("WPMZ file created successfully"),
        Err(e) => {
            println!("Error creating WPMZ: {}", e);
        }
    };
}

pub async fn create_kmz(
    waypoints: &[Waypoint],
    heading_angle: &f64,
    drone: &Drone,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = "../tmp/wpmz";
    fs::create_dir_all(dir_path)?;

    // Ensure output directory exists
    fs::create_dir_all("../output")?;

    let flightplan_path = format!("{}/flightplan.wpml", dir_path);
    let template_path = format!("{}/template.kml", dir_path);

    // Generate and write the WPML content
    let wpml_content = generate_wpml(waypoints, heading_angle, drone)?;
    fs::write(&flightplan_path, &wpml_content)?;

    // Create a basic template.kml (you might want to customize this)
    let template_content = create_template_kml()?;
    fs::write(&template_path, template_content)?;

    // Create the zip file
    let zip_path = "../output/wpmz.kmz";
    let zip_file = fs::File::create(zip_path)?;
    let mut zip = ZipWriter::new(zip_file);
    let zip_options = FileOptions::<()>::default().compression_method(Stored);

    // Add flightplan.wpml to zip
    zip.start_file("flightplan.wpml", zip_options)?;
    let flightplan_content = fs::read(&flightplan_path)?;
    zip.write_all(&flightplan_content)?;

    // Add template.kml to zip
    zip.start_file("template.kml", zip_options)?;
    let template_content = fs::read(&template_path)?;
    zip.write_all(&template_content)?;

    zip.finish()?;

    // Clean up temporary directory
    fs::remove_dir_all(dir_path)?;

    println!("Created zip file at: {}", zip_path);
    Ok(())
}

fn create_template_kml() -> Result<String, Box<dyn std::error::Error>> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    // XML declaration
    writer.write_event(Event::Decl(quick_xml::events::BytesDecl::new(
        "1.0",
        Some("UTF-8"),
        None,
    )))?;

    // Root kml element
    let mut kml_start = BytesStart::new("kml");
    kml_start.push_attribute(("xmlns", "http://www.opengis.net/kml/2.2"));
    writer.write_event(Event::Start(kml_start))?;

    // Document element
    writer.write_event(Event::Start(BytesStart::new("Document")))?;

    // Document name
    writer.write_event(Event::Start(BytesStart::new("name")))?;
    writer.write_event(Event::Text(BytesText::new("Template")))?;
    writer.write_event(Event::End(BytesEnd::new("name")))?;

    // Close document and kml
    writer.write_event(Event::End(BytesEnd::new("Document")))?;
    writer.write_event(Event::End(BytesEnd::new("kml")))?;

    // Get the written XML as a string
    let result = writer.into_inner().into_inner();
    Ok(String::from_utf8(result)?)
}

pub fn generate_wpml(
    waypoints: &[Waypoint],
    heading_angle: &f64,
    drone: &Drone,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    // XML declaration
    writer.write_event(Event::Decl(quick_xml::events::BytesDecl::new(
        "1.0",
        Some("UTF-8"),
        None,
    )))?;

    // Root kml element
    let mut kml_start = BytesStart::new("kml");
    kml_start.push_attribute(("xmlns", "http://www.opengis.net/kml/2.2"));
    kml_start.push_attribute(("xmlns:wpml", "http://www.dji.com/wpmz/1.0.2"));
    writer.write_event(Event::Start(kml_start))?;

    // Document element
    writer.write_event(Event::Start(BytesStart::new("Document")))?;

    // Mission Configuration - All required fields
    writer.write_event(Event::Start(BytesStart::new("wpml:missionConfig")))?;

    // Required: Flight mode to first waypoint
    writer.write_event(Event::Start(BytesStart::new("wpml:flyToWaylineMode")))?;
    writer.write_event(Event::Text(BytesText::new("safely")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:flyToWaylineMode")))?;

    // Required: Action after mission completion
    writer.write_event(Event::Start(BytesStart::new("wpml:finishAction")))?;
    writer.write_event(Event::Text(BytesText::new("goHome")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:finishAction")))?;

    // Required: Behavior when RC is lost
    writer.write_event(Event::Start(BytesStart::new("wpml:exitOnRCLost")))?;
    writer.write_event(Event::Text(BytesText::new("executeLostAction")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:exitOnRCLost")))?;

    // Required: RC lost action type
    writer.write_event(Event::Start(BytesStart::new("wpml:executeRCLostAction")))?;
    writer.write_event(Event::Text(BytesText::new("goBack")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:executeRCLostAction")))?;

    // Required: Safe takeoff height
    writer.write_event(Event::Start(BytesStart::new("wpml:takeOffSecurityHeight")))?;
    writer.write_event(Event::Text(BytesText::new("20")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:takeOffSecurityHeight")))?;

    // Required: Global transitional speed
    writer.write_event(Event::Start(BytesStart::new(
        "wpml:globalTransitionalSpeed",
    )))?;
    writer.write_event(Event::Text(BytesText::new(&drone.speed.to_string())))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:globalTransitionalSpeed")))?;

    // Required: Global RTH height
    writer.write_event(Event::Start(BytesStart::new("wpml:globalRTHHeight")))?;
    writer.write_event(Event::Text(BytesText::new("30")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:globalRTHHeight")))?;

    // Required: Drone information (M30 example)
    writer.write_event(Event::Start(BytesStart::new("wpml:droneInfo")))?;
    writer.write_event(Event::Start(BytesStart::new("wpml:droneEnumValue")))?;
    writer.write_event(Event::Text(BytesText::new("67")))?; // M30
    writer.write_event(Event::End(BytesEnd::new("wpml:droneEnumValue")))?;
    writer.write_event(Event::Start(BytesStart::new("wpml:droneSubEnumValue")))?;
    writer.write_event(Event::Text(BytesText::new("0")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:droneSubEnumValue")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:droneInfo")))?;

    // Required: Payload information (M30 camera)
    writer.write_event(Event::Start(BytesStart::new("wpml:payloadInfo")))?;
    writer.write_event(Event::Start(BytesStart::new("wpml:payloadEnumValue")))?;
    writer.write_event(Event::Text(BytesText::new("52")))?; // M30 camera
    writer.write_event(Event::End(BytesEnd::new("wpml:payloadEnumValue")))?;
    writer.write_event(Event::Start(BytesStart::new("wpml:payloadPositionIndex")))?;
    writer.write_event(Event::Text(BytesText::new("0")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:payloadPositionIndex")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:payloadInfo")))?;

    writer.write_event(Event::End(BytesEnd::new("wpml:missionConfig")))?;

    // Folder for waypoints with required fields
    writer.write_event(Event::Start(BytesStart::new("Folder")))?;

    // Required: Template ID
    writer.write_event(Event::Start(BytesStart::new("wpml:templateId")))?;
    writer.write_event(Event::Text(BytesText::new("0")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:templateId")))?;

    // Required: Execute height mode
    writer.write_event(Event::Start(BytesStart::new("wpml:executeHeightMode")))?;
    writer.write_event(Event::Text(BytesText::new("WGS84")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:executeHeightMode")))?;

    // Required: Wayline ID
    writer.write_event(Event::Start(BytesStart::new("wpml:waylineId")))?;
    writer.write_event(Event::Text(BytesText::new("0")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:waylineId")))?;

    // Required: Auto flight speed
    writer.write_event(Event::Start(BytesStart::new("wpml:autoFlightSpeed")))?;
    writer.write_event(Event::Text(BytesText::new(&drone.speed.to_string())))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:autoFlightSpeed")))?;

    // Gimbal pitch transition between waypoints
    writer.write_event(Event::Start(BytesStart::new("wpml:gimbalPitchMode")))?;
    writer.write_event(Event::Text(BytesText::new("usePointSetting")))?;
    writer.write_event(Event::End(BytesEnd::new("wpml:gimbalPitchMode")))?;

    // Write waypoints
    for (i, waypoint) in waypoints.iter().enumerate() {
        // Placemark for each waypoint
        writer.write_event(Event::Start(BytesStart::new("Placemark")))?;

        // Point geometry with proper coordinate format
        writer.write_event(Event::Start(BytesStart::new("Point")))?;
        writer.write_event(Event::Start(BytesStart::new("coordinates")))?;
        let coords = format!("{:.8},{:.8}", waypoint.position[0], waypoint.position[1]);
        writer.write_event(Event::Text(BytesText::new(&coords)))?;
        writer.write_event(Event::End(BytesEnd::new("coordinates")))?;
        writer.write_event(Event::End(BytesEnd::new("Point")))?;

        // Required: Waypoint index
        writer.write_event(Event::Start(BytesStart::new("wpml:index")))?;
        writer.write_event(Event::Text(BytesText::new(&i.to_string())))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:index")))?;

        // Required: Execute height
        writer.write_event(Event::Start(BytesStart::new("wpml:executeHeight")))?;
        writer.write_event(Event::Text(BytesText::new(&waypoint.altitude.to_string())))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:executeHeight")))?;

        // Required: Waypoint speed
        writer.write_event(Event::Start(BytesStart::new("wpml:waypointSpeed")))?;
        writer.write_event(Event::Text(BytesText::new(&drone.speed.to_string())))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:waypointSpeed")))?;

        // Required: Waypoint heading parameters
        writer.write_event(Event::Start(BytesStart::new("wpml:waypointHeadingParam")))?;
        writer.write_event(Event::Start(BytesStart::new("wpml:waypointHeadingMode")))?;
        writer.write_event(Event::Text(BytesText::new("fixed")))?; // Keeps it facing one direction
        writer.write_event(Event::End(BytesEnd::new("wpml:waypointHeadingMode")))?;
        writer.write_event(Event::Start(BytesStart::new("wpml:waypointHeadingAngle")))?;
        writer.write_event(Event::Text(BytesText::new(&heading_angle.to_string())))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:waypointHeadingAngle")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:waypointHeadingParam")))?;

        // Required: Waypoint turn parameters
        writer.write_event(Event::Start(BytesStart::new("wpml:waypointTurnParam")))?;
        writer.write_event(Event::Start(BytesStart::new("wpml:waypointTurnMode")))?;
        writer.write_event(Event::Text(BytesText::new(
            "toPointAndStopWithDiscontinuityCurvature",
        )))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:waypointTurnMode")))?;
        writer.write_event(Event::Start(BytesStart::new(
            "wpml:waypointTurnDampingDist",
        )))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:waypointTurnDampingDist")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:waypointTurnParam")))?;

        // Start action group
        writer.write_event(Event::Start(BytesStart::new("wpml:actionGroup")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:actionGroupStartIndex")))?;
        writer.write_event(Event::Text(BytesText::new(&i.to_string())))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:actionGroupStartIndex")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:actionGroupEndIndex")))?;
        writer.write_event(Event::Text(BytesText::new(&i.to_string())))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:actionGroupEndIndex")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:actionGroupMode")))?;
        writer.write_event(Event::Text(BytesText::new("sequence")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:actionGroupMode")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:actionTrigger")))?;
        writer.write_event(Event::Start(BytesStart::new("wpml:actionTriggerType")))?;
        writer.write_event(Event::Text(BytesText::new("reachPoint")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:actionTriggerType")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:actionTrigger")))?;

        // Gimbal rotate action
        writer.write_event(Event::Start(BytesStart::new("wpml:action")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:actionId")))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:actionId")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:actionActuatorFunc")))?;
        writer.write_event(Event::Text(BytesText::new("gimbalRotate")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:actionActuatorFunc")))?;

        writer.write_event(Event::Start(BytesStart::new(
            "wpml:actionActuatorFuncParam",
        )))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:gimbalRotateMode")))?;
        writer.write_event(Event::Text(BytesText::new("absoluteAngle")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:gimbalRotateMode")))?;

        // Pitch control
        writer.write_event(Event::Start(BytesStart::new(
            "wpml:gimbalPitchRotateEnable",
        )))?;
        writer.write_event(Event::Text(BytesText::new("1")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:gimbalPitchRotateEnable")))?;
        writer.write_event(Event::Start(BytesStart::new("wpml:gimbalPitchRotateAngle")))?;
        writer.write_event(Event::Text(BytesText::new(&waypoint.bearing.to_string())))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:gimbalPitchRotateAngle")))?;

        // Roll control
        writer.write_event(Event::Start(BytesStart::new("wpml:gimbalRollRotateEnable")))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:gimbalRollRotateEnable")))?;
        writer.write_event(Event::Start(BytesStart::new("wpml:gimbalRollRotateAngle")))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:gimbalRollRotateAngle")))?;

        // Yaw control
        writer.write_event(Event::Start(BytesStart::new("wpml:gimbalYawRotateEnable")))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:gimbalYawRotateEnable")))?;
        writer.write_event(Event::Start(BytesStart::new("wpml:gimbalYawRotateAngle")))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:gimbalYawRotateAngle")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:gimbalRotateTimeEnable")))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:gimbalRotateTimeEnable")))?;
        writer.write_event(Event::Start(BytesStart::new("wpml:gimbalRotateTime")))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:gimbalRotateTime")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:payloadPositionIndex")))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:payloadPositionIndex")))?;

        writer.write_event(Event::End(BytesEnd::new("wpml:actionActuatorFuncParam")))?;

        writer.write_event(Event::End(BytesEnd::new("wpml:action")))?;

        // Take photo action
        writer.write_event(Event::Start(BytesStart::new("wpml:action")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:actionId")))?;
        writer.write_event(Event::Text(BytesText::new("1")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:actionId")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:actionActuatorFunc")))?;
        writer.write_event(Event::Text(BytesText::new("takePhoto")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:actionActuatorFunc")))?;

        writer.write_event(Event::Start(BytesStart::new(
            "wpml:actionActuatorFuncParam",
        )))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:fileSuffix")))?;
        writer.write_event(Event::Text(BytesText::new(&i.to_string())))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:fileSuffix")))?;

        writer.write_event(Event::Start(BytesStart::new("wpml:payloadPositionIndex")))?;
        writer.write_event(Event::Text(BytesText::new("0")))?;
        writer.write_event(Event::End(BytesEnd::new("wpml:payloadPositionIndex")))?;

        writer.write_event(Event::End(BytesEnd::new("wpml:actionActuatorFuncParam")))?;

        writer.write_event(Event::End(BytesEnd::new("wpml:action")))?;

        writer.write_event(Event::End(BytesEnd::new("wpml:actionGroup")))?;

        writer.write_event(Event::End(BytesEnd::new("Placemark")))?;
    }

    // Close folder
    writer.write_event(Event::End(BytesEnd::new("Folder")))?;

    // Close document and kml
    writer.write_event(Event::End(BytesEnd::new("Document")))?;
    writer.write_event(Event::End(BytesEnd::new("kml")))?;

    // Get the written XML as a string
    let result = writer.into_inner().into_inner();
    Ok(String::from_utf8(result)?)
}
