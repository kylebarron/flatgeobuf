import { GeometryType } from '../flat-geobuf/geometry-type.js';
import type { Geometry } from '../flat-geobuf/geometry.js';

import type { ISimpleGeometry } from '../generic/geometry.js';

import LineString from 'ol/geom/LineString.js';
import MultiLineString from 'ol/geom/MultiLineString.js';
import MultiPoint from 'ol/geom/MultiPoint.js';
import MultiPolygon from 'ol/geom/MultiPolygon.js';
import Point from 'ol/geom/Point.js';
import Polygon from 'ol/geom/Polygon.js';

export function createGeometry(geometry: Geometry | null, headerGeomType: GeometryType): ISimpleGeometry | undefined {
    let geomType: GeometryType | undefined;
    if (headerGeomType === GeometryType.Unknown) {
        geomType = geometry?.type();
    } else {
        geomType = headerGeomType;
    }

    if (!geometry) return;
    const xyArray = geometry.xyArray();
    if (xyArray) {
        const xy = Array.from(xyArray);
        const ends = geometry.endsArray();
        const olEnds = ends ? Array.from(ends.map((e) => e << 1)) : [xy.length];
        switch (geomType) {
            case GeometryType.Point:
                return new Point(xy);
            case GeometryType.MultiPoint:
                return new MultiPoint(xy, 'XY');
            case GeometryType.LineString:
                return new LineString(xy, 'XY');
            case GeometryType.MultiLineString:
                return new MultiLineString(xy, 'XY', olEnds);
            case GeometryType.Polygon:
                return new Polygon(xy, 'XY', olEnds);
        }
    }
    if (geomType === GeometryType.MultiPolygon) {
        const mp = new MultiPolygon([]);
        for (let i = 0; i < geometry.partsLength(); i++)
            mp.appendPolygon(createGeometry(geometry.parts(i) as Geometry, GeometryType.Polygon) as Polygon);
        return mp;
    }
    throw new Error('Unknown type');
}
