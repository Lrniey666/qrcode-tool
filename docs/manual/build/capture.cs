// capture.exe <pid> <out.png> [client|frame]
// Saves the largest visible top-level window of process <pid> to a PNG using PrintWindow,
// so the window does not have to be in front. "client" (default) saves the client area,
// "frame" the whole window cropped to its visible bounds (title bar included).
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

static class Capture
{
    delegate bool EnumProc(IntPtr hwnd, IntPtr lParam);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr hwnd, out RECT r);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd, out RECT r);
    [DllImport("user32.dll")] static extern bool PrintWindow(IntPtr hwnd, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr hwnd, int attr, out RECT r, int size);

    [StructLayout(LayoutKind.Sequential)]
    struct RECT { public int Left, Top, Right, Bottom; }

    static int Main(string[] args)
    {
        SetProcessDpiAwarenessContext(new IntPtr(-4));
        uint pid = uint.Parse(args[0]);
        string output = args[1];
        bool frame = args.Length > 2 && args[2] == "frame";
        IntPtr best = IntPtr.Zero;
        long bestArea = 0;
        EnumWindows((h, l) =>
        {
            uint p;
            GetWindowThreadProcessId(h, out p);
            if (p != pid || !IsWindowVisible(h)) return true;
            RECT r;
            GetClientRect(h, out r);
            long area = (long)(r.Right - r.Left) * (r.Bottom - r.Top);
            if (area > bestArea) { best = h; bestArea = area; }
            return true;
        }, IntPtr.Zero);
        if (best == IntPtr.Zero) { Console.Error.WriteLine("no window"); return 2; }

        RECT rc;
        if (frame) GetWindowRect(best, out rc); else GetClientRect(best, out rc);
        int w = rc.Right - rc.Left, ht = rc.Bottom - rc.Top;
        using (var bmp = new Bitmap(w, ht, PixelFormat.Format32bppArgb))
        {
            using (var g = Graphics.FromImage(bmp))
            {
                IntPtr hdc = g.GetHdc();
                // PW_RENDERFULLCONTENT, plus PW_CLIENTONLY for the client area
                PrintWindow(best, hdc, frame ? 2u : 3u);
                g.ReleaseHdc(hdc);
            }
            Bitmap result = bmp;
            if (frame)
            {
                // DWMWA_EXTENDED_FRAME_BOUNDS: the visible frame without the resize border.
                RECT vis;
                if (DwmGetWindowAttribute(best, 9, out vis, Marshal.SizeOf(typeof(RECT))) == 0)
                {
                    var crop = new Rectangle(vis.Left - rc.Left, vis.Top - rc.Top, vis.Right - vis.Left, vis.Bottom - vis.Top);
                    result = bmp.Clone(crop, PixelFormat.Format32bppArgb);
                }
            }
            result.Save(output, ImageFormat.Png);
            Console.WriteLine(result.Width + "x" + result.Height);
        }
        return 0;
    }
}
