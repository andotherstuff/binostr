#!/usr/bin/env python3
"""Decode Rust-generated vectors with independent Python implementations."""

from __future__ import annotations

import hashlib
import importlib
import io
import json
import subprocess
import sys
import tempfile
from pathlib import Path

import avro.io
import avro.schema
import bson
import capnp
import cbor2
import msgpack
from flatbuffers import flexbuffers
from thrift.Thrift import TType
from thrift.protocol import TCompactProtocol
from thrift.transport import TTransport

ROOT = Path(__file__).resolve().parents[1]
VECTORS = json.loads((ROOT / "vectors/interop-v1.json").read_text())
EXPECTED = VECTORS["semantic_event"]


def wire(key: str) -> bytes:
    vector = VECTORS["formats"][key]
    data = bytes.fromhex(vector["bytes_hex"])
    assert hashlib.sha256(data).hexdigest() == vector["bytes_sha256"]
    return data


def event_from_sequence(value: list) -> dict:
    return {
        "id": value[0].hex(),
        "pubkey": value[1].hex(),
        "created_at": value[2],
        "kind": value[3],
        "tags": [[item.hex() if isinstance(item, bytes) else item for item in tag] for tag in value[4]],
        "content": value[5],
        "sig": value[6].hex(),
    }


def check(name: str, actual: dict) -> None:
    assert actual == EXPECTED, f"{name} mismatch:\nactual={actual!r}\nexpected={EXPECTED!r}"
    print(f"PASS {name}")


def verify_json() -> None:
    check("JSON / Python stdlib", json.loads(wire("json")))


def verify_cbor() -> None:
    check("CBOR / cbor2", event_from_sequence(cbor2.loads(wire("cbor_packed"))))


def verify_msgpack() -> None:
    check("MessagePack / msgpack-python", event_from_sequence(msgpack.unpackb(wire("msgpack"), raw=False)))


def verify_flexbuffers() -> None:
    check("FlexBuffers / flatbuffers-python", event_from_sequence(flexbuffers.Loads(wire("flexbuffers"))))


def verify_protobuf(generated: Path) -> None:
    subprocess.run(
        ["protoc", f"--python_out={generated}", "docs/nostr_binary.proto"],
        cwd=ROOT,
        check=True,
    )
    sys.path.insert(0, str(generated))
    module = importlib.import_module("docs.nostr_binary_pb2")
    value = module.ProtoEventBinary.FromString(wire("proto_binary"))
    tags = []
    for tag in value.tags:
        if tag.values_v2:
            tags.append([
                item.text if item.WhichOneof("value") == "text" else item.hex.hex()
                for item in tag.values_v2
            ])
        else:
            tags.append(list(tag.values))
    check("Protocol Buffers / google-protobuf", {
        "id": value.id.hex(), "pubkey": value.pubkey.hex(),
        "created_at": value.created_at, "kind": value.kind, "tags": tags,
        "content": value.content, "sig": value.sig.hex(),
    })


def verify_flatbuffers(generated: Path) -> None:
    subprocess.run(["flatc", "--python", "-o", str(generated), "docs/nostr.fbs"], cwd=ROOT, check=True)
    sys.path.insert(0, str(generated))
    event_type = importlib.import_module("Binostr.Event").Event
    value = event_type.GetRootAs(wire("flatbuffers"))
    tags = []
    for tag_index in range(value.TagsLength()):
        tag = value.Tags(tag_index)
        values = []
        for value_index in range(tag.ValuesLength()):
            item = tag.Values(value_index)
            text = item.Text()
            values.append(
                text.decode() if text is not None else bytes(item.Hex(i) for i in range(item.HexLength())).hex()
            )
        tags.append(values)
    check("FlatBuffers / flatbuffers-python", {
        "id": bytes(value.Id(i) for i in range(value.IdLength())).hex(),
        "pubkey": bytes(value.Pubkey(i) for i in range(value.PubkeyLength())).hex(),
        "created_at": value.CreatedAt(), "kind": value.Kind(), "tags": tags,
        "content": value.Content().decode(),
        "sig": bytes(value.Sig(i) for i in range(value.SigLength())).hex(),
    })


def verify_avro() -> None:
    schema = avro.schema.parse((ROOT / "docs/nostr.avsc").read_text())
    datum = avro.io.DatumReader(schema).read(avro.io.BinaryDecoder(io.BytesIO(wire("avro"))))
    check("Avro binary datum / apache-avro", {
        "id": datum["id"].hex(), "pubkey": datum["pubkey"].hex(),
        "created_at": datum["created_at"], "kind": datum["kind"],
        "tags": [[item.hex() if isinstance(item, bytes) else item for item in tag] for tag in datum["tags"]],
        "content": datum["content"], "sig": datum["sig"].hex(),
    })


def verify_bson() -> None:
    value = bson.BSON(wire("bson")).decode()
    check("BSON / PyMongo", {
        "id": bytes(value["id"]).hex(), "pubkey": bytes(value["pubkey"]).hex(),
        "created_at": value["created_at"], "kind": value["kind"],
        "tags": [[bytes(item).hex() if isinstance(item, (bytes, bson.binary.Binary)) else item for item in tag] for tag in value["tags"]],
        "content": value["content"], "sig": bytes(value["sig"]).hex(),
    })


def verify_capnp() -> None:
    schema = capnp.load(str(ROOT / "docs/nostr.capnp"))
    value = schema.NostrEvent.from_bytes_packed(wire("capnp_packed"))
    fixed = bytes(value.fixedData)
    tag_data = memoryview(bytes(value.tagData))
    position = 0
    tag_count = int.from_bytes(tag_data[position : position + 2], "little")
    position += 2
    tags = []
    for _ in range(tag_count):
        value_count = tag_data[position]
        position += 1
        tag = []
        for _ in range(value_count):
            header = int.from_bytes(tag_data[position : position + 2], "little")
            position += 2
            length = header & 0x7FFF
            payload = bytes(tag_data[position : position + length])
            position += length
            tag.append(payload.hex() if header & 0x8000 else payload.decode())
        tags.append(tag)
    assert position == len(tag_data)
    check("Cap'n Proto packed / pycapnp", {
        "id": fixed[:32].hex(), "pubkey": fixed[32:64].hex(),
        "created_at": int.from_bytes(fixed[128:136], "little", signed=True),
        "kind": int.from_bytes(fixed[136:138], "little"), "tags": tags,
        "content": value.content, "sig": fixed[64:128].hex(),
    })


def read_thrift_tag_value(protocol: TCompactProtocol.TCompactProtocol) -> str:
    protocol.readStructBegin()
    result = None
    while True:
        _, field_type, field_id = protocol.readFieldBegin()
        if field_type == TType.STOP:
            break
        if field_id == 1 and field_type == TType.STRING:
            result = protocol.readString()
        elif field_id == 2 and field_type == TType.STRING:
            result = protocol.readBinary().hex()
        else:
            protocol.skip(field_type)
        protocol.readFieldEnd()
    protocol.readStructEnd()
    assert result is not None
    return result


def read_thrift_tag(protocol: TCompactProtocol.TCompactProtocol) -> list[str]:
    protocol.readStructBegin()
    result = None
    while True:
        _, field_type, field_id = protocol.readFieldBegin()
        if field_type == TType.STOP:
            break
        if field_id == 1 and field_type == TType.LIST:
            element_type, size = protocol.readListBegin()
            assert element_type == TType.STRUCT
            result = [read_thrift_tag_value(protocol) for _ in range(size)]
            protocol.readListEnd()
        else:
            protocol.skip(field_type)
        protocol.readFieldEnd()
    protocol.readStructEnd()
    assert result is not None
    return result


def verify_thrift() -> None:
    transport = TTransport.TMemoryBuffer(wire("thrift_compact"))
    protocol = TCompactProtocol.TCompactProtocol(transport)
    fields = {}
    protocol.readStructBegin()
    while True:
        _, field_type, field_id = protocol.readFieldBegin()
        if field_type == TType.STOP:
            break
        if field_id in (1, 2, 7) and field_type == TType.STRING:
            fields[field_id] = protocol.readBinary().hex()
        elif field_id == 3 and field_type == TType.I64:
            fields[field_id] = protocol.readI64()
        elif field_id == 4 and field_type == TType.I16:
            fields[field_id] = protocol.readI16() & 0xFFFF
        elif field_id == 5 and field_type == TType.LIST:
            element_type, size = protocol.readListBegin()
            assert element_type == TType.STRUCT
            fields[field_id] = [read_thrift_tag(protocol) for _ in range(size)]
            protocol.readListEnd()
        elif field_id == 6 and field_type == TType.STRING:
            fields[field_id] = protocol.readString()
        else:
            protocol.skip(field_type)
        protocol.readFieldEnd()
    protocol.readStructEnd()
    assert transport.cstringio_buf.tell() == len(wire("thrift_compact"))
    check("Thrift Compact / apache-thrift", {
        "id": fields[1], "pubkey": fields[2], "created_at": fields[3],
        "kind": fields[4], "tags": fields[5], "content": fields[6], "sig": fields[7],
    })


def main() -> None:
    verify_json()
    verify_cbor()
    verify_msgpack()
    verify_flexbuffers()
    verify_avro()
    verify_bson()
    verify_capnp()
    verify_thrift()
    with tempfile.TemporaryDirectory(prefix="binostr-interop-") as directory:
        generated = Path(directory)
        verify_protobuf(generated)
        verify_flatbuffers(generated)
    print("All ten independent Python decoders matched the Rust semantic fixture.")


if __name__ == "__main__":
    main()
