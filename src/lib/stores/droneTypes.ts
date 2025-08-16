import {
    BaseDirectory,
    writeTextFile,
    readTextFile,
    mkdir,
    exists,
} from "@tauri-apps/plugin-fs";
import { type Drone } from "$lib/stores/stores";
import { resolveResource } from "@tauri-apps/api/path";

const FILE_NAME = "drone_list.json";
const DATA_DIR = BaseDirectory.AppLocalData;
const SUBDIR = "uavsar";

export async function saveDroneTypes(
    filename: string = FILE_NAME,
    droneList: Drone[],
) {
    await ensureDataDir(filename);
    const json = JSON.stringify(droneList, null, 2);
    await writeTextFile(`${SUBDIR}/${filename}`, json, { baseDir: DATA_DIR });
}

export async function loadDroneTypes(
    filename: string = FILE_NAME,
): Promise<Drone[]> {
    await ensureDataDir(filename);
    try {
        const data = await readTextFile(`${SUBDIR}/${filename}`, {
            baseDir: DATA_DIR,
        });
        return JSON.parse(data) as Drone[];
    } catch {
        console.log("Drone types file not found.");
        return [];
    }
}

async function ensureDataDir(filename: string = FILE_NAME) {
    await mkdir(SUBDIR, { baseDir: DATA_DIR, recursive: true });
    const fileExists = await exists(`${SUBDIR}/${filename}`, {
        baseDir: DATA_DIR,
    });
    if (!fileExists) {
        // Copy default drone list into DATA_DIR
        const srcPath = await resolveResource(`resources/${filename}`);
        const bytes = await readTextFile(srcPath);
        await writeTextFile(`${SUBDIR}/${filename}`, bytes, { baseDir: DATA_DIR });
    }
}
