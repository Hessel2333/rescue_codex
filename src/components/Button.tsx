import { ButtonHTMLAttributes, ReactNode } from "react";
import clsx from "clsx";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost";
  icon?: ReactNode;
};

export function Button({ className, variant = "primary", icon, children, ...props }: ButtonProps) {
  return (
    <button
      type="button"
      className={clsx(
        "button-base",
        variant === "primary" && "button-primary",
        variant === "secondary" && "button-secondary",
        variant === "ghost" && "button-ghost",
        className,
      )}
      {...props}
    >
      {icon}
      {children}
    </button>
  );
}
