declare module 'jsts/org/locationtech/jts/io/GeoJSONWriter.js' {
    export default class GeoJSONWriter {
        write(geometry: unknown): unknown;
    }
}

declare module 'jsts/org/locationtech/jts/io/WKTReader.js' {
    export default class WKTReader {
        constructor();
        read(wkt: string): unknown;
    }
}

declare module 'jsts/org/locationtech/jts/geom/Envelope.js' {
    export default class Envelope {
        constructor(minx: number, maxx: number, miny: number, maxy: number);
    }
}

declare module 'jsts/org/locationtech/jts/geom/GeometryFactory.js' {
    export default class GeometryFactory {
        toGeometry(e: unknown): unknown;
    }
}
