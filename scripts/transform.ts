import fs from "fs";
import path from "path";

function* walkSync(dir: string): IterableIterator<string> {
  const files = fs.readdirSync(dir, { withFileTypes: true });
  for (const file of files) {
    if (
      file.isDirectory() &&
      !file.name.includes("node_modules") &&
      !file.name.includes("target") &&
      !file.name.includes(".git")
    ) {
      yield* walkSync(path.join(dir, file.name));
    } else {
      yield path.join(dir, file.name);
    }
  }
}

for (const filePath of walkSync(__dirname + "/../")) {
  if (filePath.endsWith(".json")) {
    try {
      let file = fs.readFileSync(filePath);
      let json = JSON.parse(file as any);
      if (json.contract_name) {
        write(filePath, json, json.contract_name, "instantiate");
        write(filePath, json, json.contract_name, "execute");
        write(filePath, json, json.contract_name, "query");
        write(filePath, json, json.contract_name, "migrate");

        let responses = json["responses"];
        if (responses) {
          for (let key of Object.keys(responses)) {
            write(filePath, responses, json.contract_name, key);
          }
        }

        fs.unlinkSync(filePath);
      }
    } catch (error) {}
  }
  //   console.log(filePath);
}

function write(
  filePath: string,
  json: any,
  contract_name: string,
  prop: string
) {
  if (json[prop]) {
    let folder = path.dirname(filePath);
    let file =
      path.join(folder, contract_name.replace(/\-/g, "_") + "_" + prop) +
      ".json";
    console.log(file);

    let data = JSON.stringify(json[prop]);
    fs.writeFileSync(file, data);
  }

  //
}
