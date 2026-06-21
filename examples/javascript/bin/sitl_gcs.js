#!/usr/bin/env node

import * as readline from 'node:readline/promises';
import { stdin, stdout } from 'node:process';

import {
  Attitude,
  MavParamType,
  MavlinkCancellationToken,
  MavlinkCancelledException,
  MavlinkDialectRt_rc,
  MavlinkGcs,
  MavlinkTimeoutException,
} from '../../../generated/js/mavlink_protocols.js';

import { createGcsContext, gcsComponentId, gcsSystemId } from '../lib/gcs_context.js';
import { parseBaudRate, pickSerialPort } from '../lib/port_picker.js';
import { buildSampleMission, describeMissionItem } from '../lib/sample_mission.js';
import { SerialMavlinkLink } from '../lib/serial_link.js';

async function main() {
  const baudRate = parseBaudRate(process.argv.slice(2));
  const portName = await pickSerialPort();

  console.log();
  console.log(`Opening ${portName} @ ${baudRate} baud...`);

  const dialect = new MavlinkDialectRt_rc();
  const link = await SerialMavlinkLink.open(portName, baudRate);
  const gcs = MavlinkGcs.connect({
    dialect,
    link,
    systemId: gcsSystemId,
    componentId: gcsComponentId,
  });

  gcs.start();
  console.log('Publishing GCS heartbeats, waiting for vehicle...');

  let client;
  try {
    client = await gcs.waitForVehicle({
      excludeSystemIds: new Set([gcsSystemId]),
      timeoutMs: 60_000,
    });
  } catch (error) {
    if (error instanceof MavlinkTimeoutException) {
      throw new Error(
        `No vehicle heartbeat within 60 s. Check port, baud (current: ${baudRate}; try --baud 115200), and SITL.`,
      );
    }
    throw error;
  }

  const vehicle = client.vehicle;
  const vehicleState = gcs.heartbeatMonitor.stateFor(vehicle);
  console.log(`Vehicle online: ${vehicle}`);
  if (vehicleState != null) {
    console.log(
      `  type=${vehicleState.heartbeat.type} ` +
        `autopilot=${vehicleState.heartbeat.autopilot} ` +
        `status=${vehicleState.heartbeat.systemStatus}`,
    );
  }

  const ctx = createGcsContext({ gcs, vehicle, client });

  console.log();
  console.log('=== Phase 2: parameter sync ===');
  await fetchAllParameters(ctx);

  console.log();
  console.log('=== Interactive CLI ===');
  await runCli(ctx);

  console.log('Shutting down...');
  ctx.operationCancel?.cancel();
  await gcs.close();
}

async function fetchAllParameters(ctx) {
  const cancel = new MavlinkCancellationToken();
  ctx.operationCancel = cancel;

  console.log('[parameters] waiting for PARAM_VALUE stream...');
  const entries = await ctx.parameters.fetchAll({
    cancel,
    onProgress: (entry, received, expected) => {
      if (received === 1) {
        console.log(`[parameters] expecting ${expected} parameters`);
      }
      console.log(
        `[parameters] ${received}/${expected} ${entry.id}=${entry.value} (type=${entry.type})`,
      );
    },
  });
  console.log(
    `[parameters] complete (${entries.length} total, cache=${ctx.parameters.cache.size})`,
  );
}

async function runCli(ctx) {
  printHelp();

  const rl = readline.createInterface({ input: stdin, output: stdout });

  try {
    while (true) {
      const line = await rl.question('gcs> ');
      const trimmed = line.trim();
      if (!trimmed) {
        continue;
      }

      const parts = trimmed.split(/\s+/);
      const command = parts[0].toLowerCase();

      try {
        switch (command) {
          case 'h':
          case 'help':
            printHelp();
            break;
          case 'q':
          case 'quit':
          case 'exit':
            return;
          case 'hb':
            printHeartbeatStatus(ctx);
            break;
          case 'cancel':
            cancelOperation(ctx);
            break;
          case 'p':
          case 'params':
            await fetchAllParameters(ctx);
            break;
          case 'pr':
            await readParameter(ctx, parts);
            break;
          case 'pw':
            await writeParameter(ctx, parts);
            break;
          case 'mu':
            await uploadMission(ctx);
            break;
          case 'md':
            await downloadMission(ctx);
            break;
          case 'mc':
            await clearMission(ctx);
            break;
          case 'ms':
            await setMissionCurrent(ctx, parts);
            break;
          case 'rm':
            await requestMessage(ctx, parts);
            break;
          case 'si':
            await setMessageInterval(ctx, parts);
            break;
          case 'att':
            await streamAttitude(ctx, parts);
            break;
          case 'arm':
            await arm(ctx, parts);
            break;
          case 'disarm':
            await disarm(ctx, parts);
            break;
          case 'rtl':
            await returnToLaunch(ctx);
            break;
          default:
            console.log(`Unknown command: ${command} (type help)`);
        }
      } catch (error) {
        if (error instanceof MavlinkCancelledException) {
          console.log('Operation cancelled.');
        } else {
          console.log(`Error: ${error}`);
        }
      }

      console.log();
    }
  } finally {
    rl.close();
  }
}

function printHelp() {
  console.log('Commands:');
  console.log('  help              Show this help');
  console.log('  hb                Heartbeat / link status');
  console.log('  cancel            Cancel in-flight params/mission operation');
  console.log('  params            Request full parameter list (with progress)');
  console.log('  pr <name>         Read one parameter by name');
  console.log('  pw <name> <value> Write parameter (type from cache or REAL32)');
  console.log('  mu                Upload hardcoded sample mission');
  console.log('  md                Download mission from vehicle');
  console.log('  mc                Clear onboard mission');
  console.log('  ms <seq>          Set active mission item (mission + command)');
  console.log('  rm <msgId>        Request one message (MAV_CMD_REQUEST_MESSAGE)');
  console.log('  si <msgId> <us>   Set message interval (microseconds)');
  console.log('  att [seconds]     Stream ATTITUDE via listenMessage (default 5 s)');
  console.log('  arm [force]       MAV_CMD_COMPONENT_ARM_DISARM (add force for safety override)');
  console.log('  disarm [force]    Disarm motors');
  console.log('  rtl               MAV_CMD_NAV_RETURN_TO_LAUNCH');
  console.log('  quit              Exit');
}

function cancelOperation(ctx) {
  const token = ctx.operationCancel;
  if (token == null || token.isCancelled) {
    console.log('[cancel] no active cancellable operation');
    return;
  }
  token.cancel();
  console.log('[cancel] signalled');
}

function printHeartbeatStatus(ctx) {
  const node = ctx.vehicle;
  const online = ctx.heartbeatMonitor.isOnline(node);
  const state = ctx.heartbeatMonitor.stateFor(node);

  console.log(`[heartbeat] vehicle ${node} online=${online}`);
  if (state != null) {
    console.log(
      `  last=${state.ageMs}ms ago type=${state.heartbeat.type} status=${state.heartbeat.systemStatus}`,
    );
  } else {
    console.log('  no heartbeat received yet');
  }
}

async function readParameter(ctx, parts) {
  if (parts.length < 2) {
    console.log('Usage: pr <name>');
    return;
  }

  const name = parts[1];
  console.log(`[parameters] reading ${name}...`);
  const entry = await ctx.parameters.readByName(name);
  console.log(
    `[parameters] ${name}=${entry.value} (type=${entry.type}, index ${entry.index}/${entry.count})`,
  );
}

async function writeParameter(ctx, parts) {
  if (parts.length < 3) {
    console.log('Usage: pw <name> <value>');
    return;
  }

  const name = parts[1];
  const rawValue = parts[2];
  const cachedType = ctx.parameters.typeForName(name);
  const type = cachedType ?? MavParamType.MAV_PARAM_TYPE_REAL32;
  const value = parseParamValue(rawValue, type);

  console.log(`[parameters] writing ${name}=${value} (type=${type})...`);
  const entry = await ctx.parameters.writeByName(name, value);
  console.log(`[parameters] ack ${name}=${entry.value} (type=${entry.type})`);
}

function parseParamValue(raw, type) {
  switch (type) {
    case MavParamType.MAV_PARAM_TYPE_INT8:
    case MavParamType.MAV_PARAM_TYPE_INT16:
    case MavParamType.MAV_PARAM_TYPE_INT32:
    case MavParamType.MAV_PARAM_TYPE_UINT8:
    case MavParamType.MAV_PARAM_TYPE_UINT16:
    case MavParamType.MAV_PARAM_TYPE_UINT32:
      return Number.parseInt(raw, 10);
    default:
      return Number.parseFloat(raw);
  }
}

async function uploadMission(ctx) {
  const plan = buildSampleMission({
    targetSystem: ctx.targetSystem,
    targetComponent: ctx.targetComponent,
  });
  const cancel = new MavlinkCancellationToken();
  ctx.operationCancel = cancel;

  console.log(`[mission] uploading ${plan.length} hardcoded items...`);
  const result = await ctx.mission.upload(plan, {
    cancel,
    onProgress: (sent, total, item) => {
      console.log(`[mission upload] ${sent}/${total} ${describeMissionItem(item)}`);
    },
  });
  console.log(`[mission] upload finished: ${result}`);
}

async function downloadMission(ctx) {
  const cancel = new MavlinkCancellationToken();
  ctx.operationCancel = cancel;

  const items = await ctx.mission.download({
    cancel,
    onProgress: (received, total, item) => {
      console.log(`[mission download] ${received}/${total} ${describeMissionItem(item)}`);
    },
  });
  console.log('[mission] on vehicle:');
  for (const item of items) {
    console.log(`  ${describeMissionItem(item)}`);
  }
}

async function clearMission(ctx) {
  console.log('[mission] sending MISSION_CLEAR_ALL...');
  const result = await ctx.mission.clear();
  console.log(`[mission] clear result: ${result}`);
}

async function setMissionCurrent(ctx, parts) {
  if (parts.length < 2) {
    console.log('Usage: ms <seq>');
    return;
  }

  const seq = Number.parseInt(parts[1], 10);
  console.log(`[mission] set current seq=${seq} (mission + command)...`);
  const result = await ctx.mission.setCurrentWithCommand(seq, { command: ctx.command });
  console.log(
    `[mission] seq=${result.sequence} command ack=${result.commandAck?.result ?? 'n/a'}`,
  );
}

async function requestMessage(ctx, parts) {
  if (parts.length < 2) {
    console.log(`Usage: rm <msgId>  (e.g. rm ${Attitude.MSG_ID} for ATTITUDE)`);
    return;
  }

  const msgId = Number.parseInt(parts[1], 10);
  console.log(`[command] REQUEST_MESSAGE id=${msgId}`);
  const ack = await ctx.command.requestMessage(msgId);
  console.log(`[command] ack: ${ack.result}`);

  if (msgId === Attitude.MSG_ID) {
    console.log('[telemetry] waiting for ATTITUDE...');
    const attitude = await ctx.session.waitForMessageType(Attitude, {
      fromSystemId: ctx.targetSystem,
      timeoutMs: 5000,
    });
    console.log(
      `[telemetry] roll=${attitude.roll} pitch=${attitude.pitch} yaw=${attitude.yaw}`,
    );
  }
}

async function setMessageInterval(ctx, parts) {
  if (parts.length < 3) {
    console.log('Usage: si <msgId> <interval_us>  (100000 = 10 Hz, 0 = stop)');
    return;
  }

  const msgId = Number.parseInt(parts[1], 10);
  const intervalUs = Number.parseInt(parts[2], 10);
  console.log(`[command] SET_MESSAGE_INTERVAL id=${msgId} interval=${intervalUs} us`);
  const ack =
    intervalUs === 0
      ? await ctx.command.stopMessageInterval(msgId)
      : await ctx.command.setMessageInterval(msgId, intervalUs);
  console.log(`[command] ack: ${ack.result}`);
}

async function streamAttitude(ctx, parts) {
  const seconds = parts.length >= 2 ? Number.parseInt(parts[1], 10) : 5;
  console.log(`[telemetry] streaming ATTITUDE for ${seconds}s (subscribe + interval)...`);

  await ctx.command.setMessageInterval(Attitude.MSG_ID, 100_000);

  let count = 0;
  const subscription = ctx.session.listenMessage(
    (attitude) => {
      count++;
      console.log(
        `[attitude] #${count} roll=${attitude.roll.toFixed(3)} ` +
          `pitch=${attitude.pitch.toFixed(3)} yaw=${attitude.yaw.toFixed(3)}`,
      );
    },
    {
      fromSystemId: ctx.targetSystem,
      messageType: Attitude,
    },
  );

  await delay(seconds * 1000);
  subscription.cancel();
  await ctx.command.stopMessageInterval(Attitude.MSG_ID);
  console.log(`[telemetry] received ${count} ATTITUDE messages`);
}

async function arm(ctx, parts) {
  const force = parts.length >= 2 && parts[1].toLowerCase() === 'force';
  console.log(`[command] ARM${force ? ' (force)' : ''}...`);
  const ack = await ctx.command.arm({ force });
  console.log(`[command] ack: ${ack.result}`);
}

async function disarm(ctx, parts) {
  const force = parts.length >= 2 && parts[1].toLowerCase() === 'force';
  console.log(`[command] DISARM${force ? ' (force)' : ''}...`);
  const ack = await ctx.command.disarm({ force });
  console.log(`[command] ack: ${ack.result}`);
}

async function returnToLaunch(ctx) {
  console.log('[command] RETURN_TO_LAUNCH...');
  const ack = await ctx.command.returnToLaunch();
  console.log(`[command] ack: ${ack.result}`);
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
