using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Windows.Forms;

internal sealed class MaterialBackdrop : Form
{
    internal MaterialBackdrop()
    {
        Text = "NxTools Material Test Background";
        ClientSize = new Size(1100, 720);
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.FixedSingle;
        MaximizeBox = false;
        DoubleBuffered = true;
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        Rectangle area = ClientRectangle;
        using (var fill = new LinearGradientBrush(area, Color.FromArgb(108, 158, 200), Color.FromArgb(190, 104, 132), 35f))
            e.Graphics.FillRectangle(fill, area);
        using (var light = new SolidBrush(Color.FromArgb(235, 239, 248)))
        using (var dark = new SolidBrush(Color.FromArgb(32, 42, 65)))
        {
            for (int x = 0; x < area.Width; x += 32)
                e.Graphics.FillRectangle((x / 32) % 2 == 0 ? light : dark, x, 210, 16, 82);
            for (int y = 352; y < 530; y += 16)
                e.Graphics.FillRectangle((y / 16) % 2 == 0 ? light : dark, 115, y, 870, 4);
            e.Graphics.FillRectangle(light, 80, 80, 230, 85);
            e.Graphics.FillRectangle(dark, 790, 560, 230, 85);
        }
        using (var font = new Font("Microsoft YaHei UI", 14f, FontStyle.Regular))
        using (var ink = new SolidBrush(Color.FromArgb(30, 40, 60)))
        {
            e.Graphics.DrawString("NxTools · 原生材质对照背景", font, ink, 92, 100);
            e.Graphics.DrawString("静态颜色与网格；用于观察真实 DWM 模糊", font, ink, 115, 590);
        }
    }

    [STAThread]
    private static void Main()
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.Run(new MaterialBackdrop());
    }
}
