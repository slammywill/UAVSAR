import {
    BaseDirectory,
    writeTextFile,
    readTextFile,
    mkdir,
    create,
    exists,
} from "@tauri-apps/plugin-fs";
import { type Drone } from "$lib/stores/stores";

const FILE_NAME = "drone_types.json";
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
        const file = await create(`${SUBDIR}/${filename}`, { baseDir: DATA_DIR });
        await file.write(new TextEncoder().encode("[]"));
        await file.close();
    }
}
