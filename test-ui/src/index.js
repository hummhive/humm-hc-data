import { AppWebsocket } from "@holochain/conductor-api";
var Buffer = require("buffer/").Buffer;

const publicKey = "dJGu8XGhkZAil0nN2yq8Tn80aAqs5jwPJc11n1Uaa2I=";
let holochainClient;

async function init() {
  holochainClient = await AppWebsocket.connect(
    "ws://localhost:8888",
    20000,
    (signal) => handleSignal(signal, dispatch)
  );

  getDigest();
}

async function getDigest() {
  const appInfo = await holochainClient.appInfo({
    installed_app_id: "honeyworks-backup",
  });

  const cellId = appInfo.cell_data[0].cell_id;

  console.log("pub key: ", publicKey);
  const dnaHash = Buffer.from(cellId[0]).toString("base64");
  console.log("dna hash: ", dnaHash);

  const res = await holochainClient.callZome({
    cell_id: cellId,
    zome_name: "humm_hc_data",
    fn_name: "get_revision_digest",
    provenance: cellId[1],
    payload: publicKey,
    cap: null,
  });

  console.log(res);
  updateDigest(res);
}

function updateDigest(digest) {
  const element = document.getElementById("digest");
  element.innerHTML = JSON.stringify(digest, null, 4);
}

init();

const btn = document.getElementById("button");
btn.addEventListener("click", getDigest);
