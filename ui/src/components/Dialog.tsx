// Ein modaler Rahmen. Escape oder ein Klick daneben schließt.
//
// Bewusst kein <dialog> mit showModal(): jsdom kennt es nicht, und die Tests
// der Oberfläche laufen ohne Browser.

import { type ReactNode, useEffect } from "react";

type Props = {
  title: string;
  onClose: () => void;
  children: ReactNode;
};

export function Dialog({ title, onClose, children }: Props) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="backdrop" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog" role="dialog" aria-modal="true" aria-label={title}>
        <h2>{title}</h2>
        {children}
      </div>
    </div>
  );
}
