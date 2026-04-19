/* -*- Mode: JS2; indent-tabs-mode: nil; js2-basic-offset: 4 -*- */
/* vim: set et ts=4 sw=4: */
/*
 * Copyright (c) 2025 RouteMasterPro Team
 *
 * GNOME Maps is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by the
 * Free Software Foundation; either version 2 of the License, or (at your
 * option) any later version.
 *
 * GNOME Maps is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
 * or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
 * for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with GNOME Maps; if not, see <http://www.gnu.org/licenses/>.
 *
 * Offline point-to-point router backed by `rmpca route`.
 * Mirrors the GraphHopper public interface so RoutingDelegator can swap
 * between the two transparently.
 */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import gettext from 'gettext';

import {BoundingBox} from './boundingBox.js';
import {TurnPoint} from './route.js';
import {RouteQuery} from './routeQuery.js';
import * as Utils from './utils.js';

const _ = gettext.gettext;

const RMPCA_DEFAULT_PATH = 'rmpca';
const SIGTERM = 15;

/**
 * RmpcaRouter — offline point-to-point routing via `rmpca route`.
 *
 * Public interface matches GraphHopper so RoutingDelegator can use either
 * interchangeably.
 *
 * Usage:
 *   let router = new RmpcaRouter({ query, route, osmFile, rmpcaPath });
 *   // Delegator calls:
 *   router.fetchRoute(query.filledPoints, query.transportation);
 *   router.cancelCurrentRequest();
 */
export class RmpcaRouter {

    constructor({query, route, osmFile, rmpcaPath} = {}) {
        this._query      = query;
        this._route      = route;
        this._osmFile    = osmFile;
        this._rmpcaPath  = rmpcaPath || RMPCA_DEFAULT_PATH;
        this._subprocess = null;
    }

    get route() {
        return this._route;
    }

    cancelCurrentRequest() {
        if (this._subprocess) {
            this._subprocess.send_signal(SIGTERM);
            this._subprocess = null;
        }
    }

    /** Drop-in for GraphHopper.fetchRoute — updates this.route directly. */
    fetchRoute(points, transportationType) {
        this._fetchImpl(points, transportationType, (routeData, error) => {
            if (error) {
                this._route.error(error);
            } else if (routeData) {
                this._route.update(routeData);
            } else {
                this._route.error(_("No route found."));
            }
        });
    }

    /** Drop-in for GraphHopper.fetchRouteAsync — delivers result to callback. */
    fetchRouteAsync(points, transportationType, callback) {
        this._fetchImpl(points, transportationType, callback);
    }

    // ── Private ──────────────────────────────────────────────────────────────

    _fetchImpl(points, transportationType, callback) {
        if (points.length < 2) {
            callback(null, _("At least two waypoints are required"));
            return;
        }

        let from    = points[0].place.location;
        let to      = points[points.length - 1].place.location;
        let profile = this._profileFor(transportationType);

        let argv = [
            this._rmpcaPath, 'route',
            '--from',    `${from.latitude},${from.longitude}`,
            '--to',      `${to.latitude},${to.longitude}`,
            '--map',     this._osmFile,
            '--profile', profile,
        ];

        Utils.debug('RmpcaRouter: ' + argv.join(' '));

        let launcher = new Gio.SubprocessLauncher({
            flags: Gio.SubprocessFlags.STDOUT_PIPE |
                   Gio.SubprocessFlags.STDERR_PIPE,
        });

        let proc;
        try {
            proc = launcher.spawnv(argv);
            this._subprocess = proc;
        } catch (e) {
            callback(null, _("Failed to start rmpca: %s").format(e.message));
            return;
        }

        // Drain stderr for progress events (best-effort)
        this._drainLines(proc.get_stderr_pipe());

        // Read JSON result from stdout
        let buf = '';
        this._readAll(proc.get_stdout_pipe(),
            (chunk) => { buf += chunk; },
            () => {
                proc.wait_async(null, (p, res) => {
                    this._subprocess = null;
                    try {
                        p.wait_finish(res);
                    } catch (e) {
                        callback(null, e.message);
                        return;
                    }

                    let result;
                    try {
                        result = JSON.parse(buf);
                    } catch (_e) {
                        callback(null, _("Failed to parse route result"));
                        return;
                    }

                    if (!result.success) {
                        callback(null, result.error || _("Routing failed"));
                        return;
                    }

                    callback(this._buildRouteData(result), null);
                });
            });
    }

    _buildRouteData(result) {
        let path = result.path.map((pt) => ({
            latitude:  pt.latitude,
            longitude: pt.longitude,
        }));

        let bbox = new BoundingBox();
        path.forEach(({latitude, longitude}) => bbox.extend(latitude, longitude));

        return {
            path,
            turnPoints: this._buildTurnPoints(path, result.instructions),
            distance:   result.distance_m,
            time:       Math.round(result.duration_s * 1000),  // ms
            bbox,
        };
    }

    _buildTurnPoints(path, instructions) {
        return instructions.map((instr) => {
            let idx   = Math.min(instr.coordinate_index, path.length - 1);
            let coord = path[idx];
            return new TurnPoint({
                coordinate:  coord,
                type:        instr.type,
                distance:    instr.distance,
                instruction: instr.text,
                time:        0,
                turnAngle:   0,
            });
        });
    }

    /** Map RouteQuery.Transportation → rmpca profile string */
    _profileFor(transportationType) {
        switch (transportationType) {
            case RouteQuery.Transportation.CAR:        return 'car';
            case RouteQuery.Transportation.PEDESTRIAN: return 'car';  // best effort
            case RouteQuery.Transportation.BIKE:       return 'car';  // best effort
            default:                                   return 'car';
        }
    }

    /** Drain a stream line-by-line (discards content; keeps the pipe moving). */
    _drainLines(stream) {
        let ds = Gio.DataInputStream.new(stream);
        let read = () => {
            ds.read_line_async(GLib.PRIORITY_DEFAULT, null, (src, res) => {
                try {
                    let [line] = src.read_line_finish(res);
                    if (line !== null)
                        read();
                } catch (_e) { /* stream closed */ }
            });
        };
        read();
    }

    /** Read an entire GInputStream into chunks, calling onClose when EOF. */
    _readAll(stream, onData, onClose) {
        stream.read_bytes_async(4096, GLib.PRIORITY_DEFAULT, null,
            function readChunk(src, res) {
                try {
                    let bytes = src.read_bytes_finish(res);
                    if (bytes.get_size() > 0) {
                        onData(Utils.getBufferText(bytes.get_data()));
                        src.read_bytes_async(4096, GLib.PRIORITY_DEFAULT, null, readChunk);
                    } else {
                        onClose();
                    }
                } catch (_e) {
                    onClose();
                }
            });
    }
}
