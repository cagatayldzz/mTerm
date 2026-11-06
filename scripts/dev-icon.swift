import AppKit
import Foundation

let arguments = CommandLine.arguments
guard arguments.count == 3 else {
  fputs("Usage: dev-icon.swift <source.icns> <output.png>\n", stderr)
  exit(1)
}

guard let icon = NSImage(contentsOfFile: arguments[1]),
  let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 1024, pixelsHigh: 1024,
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
  let context = NSGraphicsContext(bitmapImageRep: bitmap) else {
  fputs("Could not prepare the development icon.\n", stderr)
  exit(1)
}

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = context
icon.draw(in: NSRect(x: 0, y: 0, width: 1024, height: 1024))
let shape = NSBezierPath(roundedRect: NSRect(x: 100, y: 100, width: 824, height: 824), xRadius: 185, yRadius: 185)
shape.addClip()
let ribbon = NSBezierPath()
ribbon.move(to: NSPoint(x: 100, y: 524))
ribbon.line(to: NSPoint(x: 100, y: 924))
ribbon.line(to: NSPoint(x: 500, y: 924))
ribbon.close()
NSColor(calibratedRed: 0.95, green: 0.38, blue: 0.20, alpha: 1).setFill()
ribbon.fill()
let transform = NSAffineTransform()
transform.translateX(by: 238, yBy: 707)
transform.rotate(byDegrees: 45)
transform.concat()
let attributes: [NSAttributedString.Key: Any] = [
  .font: NSFont.boldSystemFont(ofSize: 90),
  .foregroundColor: NSColor.white,
]
("DEV" as NSString).draw(at: NSPoint(x: 0, y: 0), withAttributes: attributes)
NSGraphicsContext.restoreGraphicsState()

guard let png = bitmap.representation(using: .png, properties: [:]) else {
  fputs("Could not encode the development icon.\n", stderr)
  exit(1)
}
try png.write(to: URL(fileURLWithPath: arguments[2]))
