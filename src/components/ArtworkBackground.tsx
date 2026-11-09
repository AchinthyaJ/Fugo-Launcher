/**
 * The launcher's background. `src` picks the image; the default is backgrounds/1.png.
 */
export default function ArtworkBackground({ src = "/assets/backgrounds/1.png" }: { src?: string }) {
  return (
    <div aria-hidden="true" className="absolute inset-0 z-0 overflow-hidden pointer-events-none bg-[#0B0D14]">
      <img src={src} alt="" draggable={false} className="w-full h-full object-cover object-center scale-105" />
      {/* Darkened subtle treatments so text remains readable */}
      <div className="absolute inset-0 bg-gradient-to-t from-[#0B0D14] via-transparent to-[#0B0D14]/80" />
      <div className="absolute inset-0 bg-gradient-to-r from-[#0B0D14]/90 via-transparent to-[#0B0D14]/75" />
    </div>
  );
}
