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
 * BBoxSelector — click-drag bounding box selection on the map.
 * Renders a rectangle overlay using Shumate.PathLayer and suppresses
 * map panning while the user is dragging.
 */

import Gdk from 'gi://Gdk';
import Gtk from 'gi://Gtk';
import Shumate from 'gi://Shumate';

import {BoundingBox} from './boundingBox.js';

/* Orange outline for the selection rectangle */
const SELECTION_STROKE = new Gdk.RGBA({ red:   0xff / 255,
                                        green: 0x78 / 255,
                                        blue:  0x00 / 255,
                                        alpha: 0.9 });

/**
 * BBoxSelector — lets the user draw a bounding box on the map.
 *
 * Usage:
 *   let selector = new BBoxSelector(mapView);
 *   selector.enable((bbox) => { ... use the BoundingBox ... });
 *   selector.disable();
 */
export class BBoxSelector {

    constructor(mapView) {
        this._mapView = mapView;
        this._active = false;
        this._dragGesture = null;
        this._bboxLayer = null;
        this._startX = 0;
        this._startY = 0;
        this._callback = null;
    }

    get active() {
        return this._active;
    }

    /**
     * Activate bbox selection mode.
     * @param {function} callback - Called with a BoundingBox when the user
     *                              finishes dragging.
     */
    enable(callback) {
        if (this._active)
            return;

        this._active = true;
        this._callback = callback;

        /* Create a drag gesture on the Shumate.Map widget */
        this._dragGesture = new Gtk.GestureDrag();
        this._dragGesture.connect('drag-begin', this._onDragBegin.bind(this));
        this._dragGesture.connect('drag-update', this._onDragUpdate.bind(this));
        this._dragGesture.connect('drag-end',   this._onDragEnd.bind(this));
        this._mapView.map.add_controller(this._dragGesture);

        /* Path layer for the selection rectangle */
        this._bboxLayer = new Shumate.PathLayer({
            viewport:      this._mapView.map.viewport,
            stroke_width:  2,
            stroke_color:  SELECTION_STROKE,
        });
        this._mapView.map.insert_layer_above(this._bboxLayer,
                                              this._mapView._mapLayer);
    }

    /**
     * Deactivate bbox selection and clean up.
     */
    disable() {
        if (!this._active)
            return;

        this._active = false;
        this._callback = null;

        if (this._dragGesture) {
            this._mapView.map.remove_controller(this._dragGesture);
            this._dragGesture = null;
        }

        if (this._bboxLayer) {
            this._bboxLayer.remove_all();
            this._mapView.map.remove_layer(this._bboxLayer);
            this._bboxLayer = null;
        }
    }

    /* ---- Gesture handlers ------------------------------------------------ */

    _onDragBegin(gesture, startX, startY) {
        /* Claim the event sequence so the map doesn't pan */
        gesture.set_state(Gtk.EventSequenceState.CLAIMED);
        this._startX = startX;
        this._startY = startY;
        this._bboxLayer.remove_all();
    }

    _onDragUpdate(gesture, offsetX, offsetY) {
        this._updateRect(this._startX, this._startY,
                         this._startX + offsetX, this._startY + offsetY);
    }

    _onDragEnd(gesture, offsetX, offsetY) {
        let endX = this._startX + offsetX;
        let endY = this._startY + offsetY;

        let viewport = this._mapView.map.viewport;
        let [lat1, lon1] = viewport.widget_coords_to_location(this._mapView,
                                                               this._startX,
                                                               this._startY);
        let [lat2, lon2] = viewport.widget_coords_to_location(this._mapView,
                                                               endX, endY);

        let bbox = new BoundingBox({
            left:   Math.min(lon1, lon2),
            right:  Math.max(lon1, lon2),
            bottom: Math.min(lat1, lat2),
            top:    Math.max(lat1, lat2),
        });

        if (this._callback)
            this._callback(bbox);
    }

    /* ---- Rectangle rendering --------------------------------------------- */

    _updateRect(x1, y1, x2, y2) {
        this._bboxLayer.remove_all();

        let viewport = this._mapView.map.viewport;
        let [lat1, lon1] = viewport.widget_coords_to_location(this._mapView, x1, y1);
        let [lat2, lon2] = viewport.widget_coords_to_location(this._mapView, x2, y2);

        let west  = Math.min(lon1, lon2);
        let east  = Math.max(lon1, lon2);
        let south = Math.min(lat1, lat2);
        let north = Math.max(lat1, lat2);

        /* Five points to close the rectangle loop */
        let corners = [
            [north, west], [north, east],
            [south, east], [south, west],
            [north, west],
        ];

        for (let [lat, lon] of corners) {
            let marker = new Shumate.Marker({ latitude:  lat,
                                              longitude: lon,
                                              visible:   false });
            this._bboxLayer.add_node(marker);
        }
    }
}
